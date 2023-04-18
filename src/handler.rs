use crate::{
    model::{AppState, RequestBody},
    response::JsonResponse,
    serverless::*,
};

use actix_web::{post, web, HttpResponse, Responder};
use serde_json::Value;
use std::env;
use std::io::{BufRead, BufReader};
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
    log::info!("*********NEW**REQUEST*******");
    // Validation for the request json body
    if let Err(err) = jsonbody.validate() {
        log::error!("{}", err);
        let resp = JsonResponse {
            status: "error".to_string(),
            message: "Invalid payload".to_string(),
            data: Some(Value::String(err.to_string())),
        };
        return HttpResponse::BadRequest().json(resp);
    }

    let workerd_runtime_path = env::var("RUNTIME_PATH").expect("RUNTIME_PATH must be a valid path");
    let tx_hash = jsonbody.tx_hash.as_ref().unwrap();

    //Creating a unique file name for the output file
    let file_name = tx_hash.to_string() + &Uuid::new_v4().to_string();

    //Fetching the transaction data using the transaction hash and decoding the calldata
    let json_response = match get_transaction_data(tx_hash).await {
        Ok(data) => data,
        Err(e) => {
            let resp = JsonResponse {
                status: "error".to_string(),
                message: "Error fetching transacton data".to_string(),
                data: None,
            };
            log::error!("Error : {}", e);
            return HttpResponse::InternalServerError().json(resp);
        }
    };

    let call_data = json_response["result"]["input"].to_string();
    let contract_address = json_response["result"]["to"].to_string();

    //Checking if the contract address is correct
    if contract_address != "\"0x30694a76d737211a908d0dd672f47e1d29fbfb02\"" {
        let resp = JsonResponse {
            status: "error".to_string(),
            message: "Please make sure you are interacting with the correct contract : 0x30694a76d737211a908d0dd672f47e1d29fbfb02"
                .to_string(),
            data: None,
        };
        return HttpResponse::BadRequest().json(resp);
    }

    //Checking if the call data is null
    if call_data == "null" {
        let resp = JsonResponse {
            status: "error".to_string(),
            message: "Error fetching the call data, make sure a valid tx_hash is provided"
                .to_string(),
            data: None,
        };
        return HttpResponse::BadRequest().json(resp);
    }

    let execution_timer_start = Instant::now();

    let decoded_calldata = match decode_call_data(&call_data) {
        Ok(data) => data,
        Err(e) => {
            log::error!("{}", e);
            let resp = JsonResponse {
                status: "error".to_string(),
                message: "Error decoding the call data".to_string(),
                data: None,
            };
            return HttpResponse::InternalServerError().json(resp);
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
            let resp = JsonResponse {
                status: "error".to_string(),
                message: "Error generating the JS file".to_string(),
                data: None,
            };
            log::error!("Error : {}", e);
            return HttpResponse::InternalServerError().json(resp);
        }
    };

    let capnp_file = create_capnp_file(&file_name, free_port, &workerd_runtime_path).await;

    match capnp_file {
        Ok(_) => {
            log::info!("Config file generated.")
        }
        Err(e) => {
            let resp = JsonResponse {
                status: "error".to_string(),
                message: "Error generating the configuration file".to_string(),
                data: None,
            };
            log::error!("Error : {}", e);
            return HttpResponse::InternalServerError().json(resp);
        }
    }

    let js_file_path = workerd_runtime_path.to_string() + &file_name.to_string() + ".js";
    let capnp_file_path = workerd_runtime_path.to_string() + &file_name.to_string() + ".capnp";

    //Finding an available cgroup
    let cgroup_list = &appstate.cgroup_list;
    let available_cgroup = match find_available_cgroup(appstate.cgroup_version, cgroup_list) {
        Ok(cgroup) => cgroup,
        Err(e) => {
            log::error!("{}", e);
            return internal_server_error_response(
                &capnp_file_path,
                &js_file_path,
                "There was an error assigning resources to your function",
            );
        }
    };

    if available_cgroup == "No available cgroup" {
        log::error!("No available cgroup to run workerd");
        return internal_server_error_response(&capnp_file_path, &js_file_path, "Server busy");
    }

    //Run the workerd runtime with generated files

    let workerd = run_workerd_runtime(&file_name, &workerd_runtime_path, &available_cgroup).await;

    if workerd.is_err() {
        let workerd_error = workerd.err();
        log::error!("Error running the workerd runtime: {:?}", workerd_error);
        return internal_server_error_response(
            &capnp_file_path,
            &js_file_path,
            "Error running the workerd runtime",
        );
    }

    let mut workerd_process = match workerd {
        Ok(data) => data,
        Err(e) => {
            log::error!("{}", e);
            return internal_server_error_response(
                &capnp_file_path,
                &js_file_path,
                "Failed to execute the code",
            );
        }
    };

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
                let resp = handle_workerd_error(
                    &capnp_file_path,
                    &js_file_path,
                    &mut workerd_process,
                    "Server timeout, fetching response took a long time",
                );
                return HttpResponse::RequestTimeout().json(resp);
            }
        };

        let workerd_response = match workerd_response_with_timeoutcheck {
            Ok(res) => res,
            Err(err) => {
                log::error!("workerd response error: {}", err);
                let resp = handle_workerd_error(
                    &capnp_file_path,
                    &js_file_path,
                    &mut workerd_process,
                    "Failed to generate the response",
                );
                return HttpResponse::InternalServerError().json(resp);
            }
        };

        if workerd_response.status() != reqwest::StatusCode::OK {
            let resp = handle_workerd_error(
                &capnp_file_path,
                &js_file_path,
                &mut workerd_process,
                "The server failed to retrieve a response. Please ensure that you have implemented appropriate exception handling in your JavaScript code."
            );
            return HttpResponse::InternalServerError().json(resp);
        }

        let workerd_json_response = workerd_response.text().await.unwrap();

        //Terminating the workerd process once the response is fetched
        let kill_workerd_process = workerd_process.kill();

        match kill_workerd_process {
            Ok(_) => {
                log::info!("Workerd process {} terminated.", workerd_process.id())
            }
            Err(_) => {
                log::error!("Error terminating the process : {}", workerd_process.id())
            }
        }

        //Delete the generated file once the response is generated

        delete_file(&js_file_path).expect("Error deleting JS file");
        delete_file(&capnp_file_path).expect("Error deleting configuration file");

        let resp = JsonResponse {
            status: "success".to_string(),
            message: "Response successfully generated".to_string(),
            data: Some(Value::String(workerd_json_response)),
        };

        log::info!("Generated response");
        let execution_timer_end = Instant::now();
        let execution_time = execution_timer_end
            .duration_since(execution_timer_start)
            .as_millis()
            .to_string();
        log::info!("Execution time: {}ms", execution_time);
        HttpResponse::Ok().json(resp)
    } else {
        let stderr = workerd_process.stderr.take().unwrap();
        let reader = BufReader::new(stderr);

        let stderr_lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
        let stderr_output = stderr_lines.join("\n");

        if !stderr_output.is_empty() {
            log::error!("Workerd execution error : {}", stderr_output);

            if stderr_output.contains("SyntaxError") {
                delete_file(&js_file_path).expect("Error deleting JS file");
                delete_file(&capnp_file_path).expect("Error deleting configuration file");
                let resp = JsonResponse {
                    status: "error".to_string(),
                    message:String::from("Failed to generate a response. Syntax error in your JavaScript code. Please check the syntax and try again."),
                    data: Some(Value::String(stderr_output))
                };
                return HttpResponse::BadRequest().json(resp);
            }

            return internal_server_error_response(
                &capnp_file_path,
                &js_file_path,
                "Failed to generate a response.",
            );
        }

        let workerd_status = workerd_process.try_wait();
        match workerd_status {
            Ok(status) => {
                let error_status = status.unwrap().to_string();
                log::error!("Workerd execution error : {}", error_status);
                if error_status == "signal: 9 (SIGKILL)" {
                    return internal_server_error_response(
                        &capnp_file_path,
                        &js_file_path,
                        "The execution of the code has run out of memory.",
                    );
                }
            }
            Err(err) => panic!("Error fetching workerd exit status : {}", err),
        }

        internal_server_error_response(
            &capnp_file_path,
            &js_file_path,
            "Failed to generate a response.",
        )
    }
}

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api").service(serverless);
    conf.service(scope);
}
