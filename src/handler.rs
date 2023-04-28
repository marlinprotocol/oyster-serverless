use crate::{
    model::{AppState, RequestBody},
    response::response,
    serverless::*,
};

use actix_web::http::StatusCode;
use actix_web::{post, web, Responder};
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
        return response(
            None,
            None,
            None,
            Some(Value::String(err.to_string())),
            "Invalid payload",
            StatusCode::BAD_REQUEST,
        );
    }

    let workerd_runtime_path = env::var("RUNTIME_PATH").expect("RUNTIME_PATH must be a valid path");
    let code_id = jsonbody.code_id.as_ref().unwrap();

    //Creating a unique file name for the output file
    let file_name = code_id.to_string() + &Uuid::new_v4().to_string();

    //Fetching attestation document
    let attestation_document = match get_attestation_doc().await {
        Ok(attestation_doc) => attestation_doc.text().await.unwrap(),
        Err(e) => {
            log::error!("{}", e);
            return response(
                None,
                None,
                None,
                None,
                "There was a problem in generating the attestation document",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    //encode the attestation doc into base64
    let base64_encoded_attestation_doc = base64::encode(&attestation_document);
    //Fetching the js code from the storage server
    let js_code = match get_code_from_storage_server(&base64_encoded_attestation_doc, code_id).await
    {
        Ok(code) => {
            if code.status() == StatusCode::OK {
                code.text().await.unwrap()
            } else {
                return response(
                    None,
                    None,
                    None,
                    None,
                    "There was a problem in fetching the code from the storage server",
                    StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
        }
        Err(e) => {
            log::error!("{}", e);
            return response(
                None,
                None,
                None,
                None,
                "There was a problem in fetching the code from the storage server",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let execution_timer_start = Instant::now();

    //Fetching a free port
    let free_port = get_free_port();
    log::info!("Free port: {}", &free_port);

    //Creating file names
    let js_file_path = workerd_runtime_path.to_string() + &file_name.to_string() + ".js";
    let capnp_file_path = workerd_runtime_path.to_string() + &file_name.to_string() + ".capnp";

    //Generating the js and capnp file
    let js_file = create_js_file(&js_code, &js_file_path).await;

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

    let capnp_file = create_capnp_file(&file_name, free_port, &capnp_file_path).await;

    match capnp_file {
        Ok(_) => {
            log::info!("Config file generated.")
        }
        Err(e) => {
            log::error!("Error : {}", e);
            return response(
                None,
                Some(&js_file_path),
                None,
                None,
                "Error generating the configuration file.",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    }

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

    // Wait for the port to bind
    if wait_for_port(free_port) {
        //Fetching the workerd response with 30sec timeout
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

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api").service(serverless);
    conf.service(scope);
}
