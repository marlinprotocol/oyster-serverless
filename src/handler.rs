use crate::{
    model::{AppState, RequestBody},
    response::response,
    serverless::*,
};

use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpResponse, Responder};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use tokio::time::timeout;
use uuid::Uuid;
use validator::Validate;

#[post("/serverless")]
async fn serverless(
    jsonbody: web::Json<RequestBody>,
    appstate: web::Data<AppState>,
) -> impl Responder {
    // check if the server is draining
    // IMPORTANT: we use Relaxed ordering here since we do not need to synchronize any memory
    // not even with reads/writes to the same atomic (we just serve a few more requests at worst)
    // be very careful adding more operations associated with the draining state
    if !appstate.running.load(Ordering::Relaxed) {
        return HttpResponse::Gone().body("worker unregistered");
    }

    // validate request body
    if let Err(err) = jsonbody.validate() {
        return HttpResponse::BadRequest().body(format!("invalid payload: {err:?}"));
    }

    let workerd_runtime_path = appstate.runtime_path.clone();
    let tx_hash = &jsonbody.tx_hash;

    //Creating a unique file name for the output file
    let file_name = tx_hash.to_string() + &Uuid::new_v4().to_string();

    //Fetching the transaction data using the transaction hash and decoding the calldata
    let json_response = match get_transaction_data(tx_hash).await {
        Ok(data) => data,
        Err(e) => {
            log::error!("Error : {}", e);
            return response(
                None,
                None,
                None,
                None,
                "Error fetching transacton data",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let call_data = json_response["result"]["input"].to_string();
    let contract_address = json_response["result"]["to"].to_string();

    //Checking if the contract address is correct
    if contract_address != "\"0x30694a76d737211a908d0dd672f47e1d29fbfb02\"" {
        return response(
            None,
            None,
            None,
            None,
            "Please make sure you are interacting with the correct contract : 0x30694a76d737211a908d0dd672f47e1d29fbfb02",
            StatusCode::BAD_REQUEST,
        );
    }

    //Checking if the call data is null
    if call_data == "null" {
        return response(
            None,
            None,
            None,
            None,
            "Error fetching the call data, make sure a valid tx_hash is provided",
            StatusCode::BAD_REQUEST,
        );
    }

    let execution_timer_start = Instant::now();

    let decoded_calldata = match decode_call_data(&call_data) {
        Ok(data) => data,
        Err(e) => {
            log::error!("{}", e);
            return response(
                None,
                None,
                None,
                None,
                "Error decoding the call data",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    //Fetching a free port
    let free_port = get_free_port();
    log::info!("Free port: {}", &free_port);

    //Generating the js and capnp file
    let js_file = create_js_file(&decoded_calldata, &file_name, &workerd_runtime_path).await;

    match js_file {
        Ok(_) => {
            log::info!("JS file generated.")
        }
        Err(e) => {
            log::error!("Error : {}", e);
            return response(
                None,
                None,
                None,
                None,
                "Error generating the JS file",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let capnp_file = create_capnp_file(&file_name, free_port, &workerd_runtime_path).await;

    match capnp_file {
        Ok(_) => {
            log::info!("Config file generated.")
        }
        Err(e) => {
            log::error!("Error : {}", e);
            return response(
                None,
                None,
                None,
                None,
                "Error generating the configuration file.",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    }

    // return response(
    //     None,
    //     None,
    //     None,
    //     None,
    //     "Error generating the configuration file.",
    //     StatusCode::INTERNAL_SERVER_ERROR,
    // );

    let js_file_path = workerd_runtime_path.to_string() + &file_name.to_string() + ".js";
    let capnp_file_path = workerd_runtime_path.to_string() + &file_name.to_string() + ".capnp";

    //Finding an available cgroup
    let cgroup_list = &appstate.cgroup_list;
    let available_cgroup = match find_available_cgroup(appstate.cgroup_version, cgroup_list) {
        Ok(cgroup) => cgroup,
        Err(e) => {
            log::error!("{}", e);
            return response(
                Some(&capnp_file_path),
                Some(&js_file_path),
                None,
                None,
                "There was an error assigning resources to your function",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    if available_cgroup == "No available cgroup" {
        log::error!("No available cgroup to run workerd");
        return response(
            Some(&capnp_file_path),
            Some(&js_file_path),
            None,
            None,
            "Server busy",
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    }
    //Run the workerd runtime with generated files

    let workerd = run_workerd_runtime(&file_name, &workerd_runtime_path, &available_cgroup).await;

    // let wrkr = match workerd {
    //     Ok(child) => child,
    //     Err(e) => {
    //         return response(
    //             Some(&capnp_file_path),
    //             Some(&js_file_path),
    //             None,
    //             None,
    //             "Error running the workerd runtime",
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //         );
    //     }
    // };

    // if let Some(wrkr_err) = wrkr.stderr {
    //     println!("{:?}", wrkr_err);
    // };

    if workerd.is_err() {
        let workerd_error = workerd.err();
        log::error!("Error running the workerd runtime: {:?}", workerd_error);
        return response(
            Some(&capnp_file_path),
            Some(&js_file_path),
            None,
            None,
            "Error running the workerd runtime",
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    }

    let mut workerd_process = match workerd {
        Ok(data) => data,
        Err(e) => {
            log::error!("{}", e);
            return response(
                Some(&capnp_file_path),
                Some(&js_file_path),
                None,
                None,
                "Failed to execute the code",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };
    // println!("{:?}", workerd_process);
    // Wait for the port to bind
    if wait_for_port(free_port) {
        //Fetching the workerd response
        let api_response_with_timeout = timeout(
            Duration::from_secs(30),
            get_workerd_response(free_port, jsonbody.input.as_ref().cloned()),
        )
        .await;

        let workerd_response_with_timeoutcheck = match api_response_with_timeout {
            Ok(response) => response,
            Err(err) => {
                log::error!("workerd response error: {}", err);
                log::error!("Failed to fetch response from workerd in 30sec");
                return response(
                    Some(&capnp_file_path),
                    Some(&js_file_path),
                    Some(workerd_process),
                    None,
                    "Server timeout, fetching response took a long time",
                    StatusCode::REQUEST_TIMEOUT,
                );
            }
        };

        let workerd_response = match workerd_response_with_timeoutcheck {
            Ok(res) => res,
            Err(err) => {
                log::error!("workerd response error: {}", err);
                return response(
                    Some(&capnp_file_path),
                    Some(&js_file_path),
                    Some(workerd_process),
                    None,
                    "Failed to generate the response",
                    StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
        };

        if workerd_response.status() != reqwest::StatusCode::OK {
            return response(
                Some(&capnp_file_path),
                Some(&js_file_path),
                Some(workerd_process),
                None,
                "The server failed to retrieve a response. Please ensure that you have implemented appropriate exception handling in your JavaScript code.",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }

        let workerd_json_response = workerd_response.text().await.unwrap();

        log::info!("Generated response");
        let execution_timer_end = Instant::now();
        let execution_time = execution_timer_end
            .duration_since(execution_timer_start)
            .as_millis()
            .to_string();
        log::info!("Execution time: {}ms", execution_time);

        response(
            Some(&capnp_file_path),
            Some(&js_file_path),
            Some(workerd_process),
            Some(Value::String(workerd_json_response)),
            "Response successfully generated",
            StatusCode::OK,
        )
    } else {
        let stderr = workerd_process.stderr.take().unwrap();
        let reader = BufReader::new(stderr);

        let stderr_lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
        let stderr_output = stderr_lines.join("\n");

        if !stderr_output.is_empty() {
            log::error!("Workerd execution error : {}", stderr_output);

            if stderr_output.contains("SyntaxError") {
                return response(
                    Some(&capnp_file_path),
                    Some(&js_file_path),
                    None,
                    Some(Value::String(stderr_output)),
                    "Failed to generate a response. Syntax error in your JavaScript code. Please check the syntax and try again.",
                    StatusCode::BAD_REQUEST,
                );
            }

            return response(
                Some(&capnp_file_path),
                Some(&js_file_path),
                None,
                None,
                "Failed to generate a response.",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }

        let workerd_status = workerd_process.try_wait();
        match workerd_status {
            Ok(status) => {
                let error_status = status.unwrap().to_string();
                log::error!("Workerd execution error : {}", error_status);
                if error_status == "signal: 9 (SIGKILL)" {
                    return response(
                        Some(&capnp_file_path),
                        Some(&js_file_path),
                        None,
                        None,
                        "The execution of the code has run out of memory.",
                        StatusCode::INTERNAL_SERVER_ERROR,
                    );
                }
            }
            Err(err) => log::error!("Error fetching workerd exit status : {}", err),
        }

        response(
            Some(&capnp_file_path),
            Some(&js_file_path),
            None,
            None,
            "Failed to generate a response.",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    }
}

#[get("/unregister")]
async fn unregister(appstate: web::Data<AppState>) -> impl Responder {
    // IMPORTANT: we use Relaxed ordering here since we do not need to synchronize any memory
    // not even with reads/writes to the same atomic (we just serve a few more requests at worst)
    // be very careful adding more operations associated with the draining state
    appstate.running.store(false, Ordering::Relaxed);

    return HttpResponse::Ok()
        .status(StatusCode::OK)
        .body("successfully set server in draining state");
}

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api").service(serverless).service(unregister);
    conf.service(scope);
}
