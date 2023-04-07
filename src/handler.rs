use crate::{
    model::{AppState, RequestBody, SystemInfo},
    response::JsonResponse,
    serverless::*,
};

use actix_web::{get, post, web, HttpResponse, Responder};
use serde_json::Value;
use std::env;
use std::io::{BufRead, BufReader};
use std::time::Instant;
use sysinfo::{System, SystemExt};
use uuid::Uuid;
use validator::Validate;

#[get("/serverlessinfo")]
async fn serverlessinfo() -> HttpResponse {
    let mut system = System::new_all();
    system.refresh_all();

    let free_memory = system.available_memory();
    let total_system_memory = system.total_memory();

    let sys_info = SystemInfo {
        free_memory,
        total_system_memory,
    };

    HttpResponse::Ok().json(sys_info)
}

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

    let file_name = tx_hash.to_string() + &Uuid::new_v4().to_string();

    let fetch_timer_start = Instant::now();

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

    let fetch_timer_end = Instant::now();
    let fetch_time = fetch_timer_end.duration_since(fetch_timer_start);

    log::info!("Time taken to fetch data : {:?}", fetch_time);

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
            let resp = JsonResponse {
                status: "error".to_string(),
                message: "There was an error assigning resources to your function".to_string(),
                data: None,
            };

            delete_file(&js_file_path).expect("Error deleting JS file");
            delete_file(&capnp_file_path).expect("Error deleting configuration file");

            return HttpResponse::InternalServerError().json(resp);
        }
    };

    if available_cgroup == "No available cgroup" {
        let resp = JsonResponse {
            status: "error".to_string(),
            message: "Server busy".to_string(),
            data: None,
        };
        log::error!("No available cgroup to run workerd");
        delete_file(&js_file_path).expect("Error deleting JS file");
        delete_file(&capnp_file_path).expect("Error deleting configuration file");

        return HttpResponse::InternalServerError().json(resp);
    }

    //Run the workerd runtime with generated files

    let workerd_execution_start = Instant::now();
    let workerd = run_workerd_runtime(&file_name, &workerd_runtime_path, &available_cgroup).await;

    if workerd.is_err() {
        delete_file(&js_file_path).expect("Error deleting JS file");
        delete_file(&capnp_file_path).expect("Error deleting configuration file");
        let workerd_error = workerd.err();
        log::error!("Error running the workerd runtime: {:?}", workerd_error);
        let resp = JsonResponse {
            status: "error".to_string(),
            message: "Error running the workerd runtime".to_string(),
            data: None,
        };
        return HttpResponse::InternalServerError().json(resp);
    }

    let mut workerd_process = match workerd {
        Ok(data) => data,
        Err(e) => {
            log::error!("{}", e);
            delete_file(&js_file_path).expect("Error deleting JS file");
            delete_file(&capnp_file_path).expect("Error deleting configuration file");
            let resp = JsonResponse {
                status: "error".to_string(),
                message: "Failed to execute the code".to_string(),
                data: None,
            };
            return HttpResponse::InternalServerError().json(resp);
        }
    };

    // Wait for the port to bind
    if wait_for_port(free_port) {
        //Fetching the workerd response
        let workerd_response = get_workerd_response(free_port, jsonbody.input.as_ref().cloned())
            .await
            .unwrap();

        if workerd_response.status() != reqwest::StatusCode::OK {
            delete_file(&js_file_path).expect("Error deleting JS file");
            delete_file(&capnp_file_path).expect("Error deleting configuration file");
            let resp = JsonResponse {
                status: "error".to_string(),
                message: "The server failed to retrieve a response. Please ensure that you have implemented appropriate exception handling in your JavaScript code.".to_string(),
                data: None,
            };
            return HttpResponse::InternalServerError().json(resp);
        }

        let workerd_json_response = workerd_response.text().await.unwrap();

        //Terminating the workerd process once the response is fetched
        let kill_workerd_process = workerd_process.kill();

        //Fetching the workerd execution duration
        let workerd_execution_end = Instant::now();
        let workerd_execution_duration = workerd_execution_end
            .duration_since(workerd_execution_start)
            .as_millis()
            .to_string();
        log::info!("Workerd execution time: {}ms", workerd_execution_duration);

        match kill_workerd_process {
            Ok(_) => {
                log::info!("Workerd process {} terminated.", workerd_process.id())
            }
            Err(_) => {
                log::error!("Error terminating the process : {}", workerd_process.id())
            }
        }

        //Delete the generated file once the response is generated

        let _deleted_js_file = delete_file(&js_file_path);
        let _deleted_capnp_file = delete_file(&capnp_file_path);

        let resp = JsonResponse {
            status: "success".to_string(),
            message: "Response successfully generated".to_string(),
            data: Some(Value::String(workerd_json_response)),
        };

        log::info!("Generated response");
        HttpResponse::Ok().json(resp)
    } else {
        let stderr = workerd_process.stderr.take().unwrap();
        let reader = BufReader::new(stderr);

        let stderr_lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
        let stderr_output = stderr_lines.join("\n");

        if !stderr_output.is_empty() {
            let resp = JsonResponse {
                status: "error".to_string(),
                message:
                    "Failed to generate a response. Please ensure that you provide valid JS code"
                        .to_string(),
                data: Some(Value::String(stderr_output)),
            };

            delete_file(&js_file_path).expect("Error deleting JS file");
            delete_file(&capnp_file_path).expect("Error deleting configuration file");
            return HttpResponse::InternalServerError().json(resp);
        }

        let workerd_status = workerd_process.try_wait();
        match workerd_status {
            Ok(status) => {
                let error_status = status.unwrap().to_string();
                log::error!("Workerd execution error : {}", error_status);
                if error_status == "signal: 9 (SIGKILL)" {
                    let resp = JsonResponse {
                        status: "error".to_string(),
                        message: "Workerd ran out of memory".to_string(),
                        data: None,
                    };
                    return HttpResponse::InternalServerError().json(resp);
                }
            }
            Err(err) => panic!("Error fetching workerd exit status : {}", err),
        }

        delete_file(&js_file_path).expect("Error deleting JS file");
        delete_file(&capnp_file_path).expect("Error deleting configuration file");

        let resp = JsonResponse {
            status: "error".to_string(),
            message: "Failed to generate a response. Please ensure that you provide valid JS code."
                .to_string(),
            data: None,
        };
        HttpResponse::InternalServerError().json(resp)
    }
}

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api")
        .service(serverless)
        .service(serverlessinfo);
    conf.service(scope);
}
