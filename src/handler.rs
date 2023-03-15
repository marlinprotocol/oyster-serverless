use crate::{
    model::RequestBody,
    response::{JsonResponse},
    serverless::*,
};

use actix_web::{get, web, HttpResponse, Responder};
use psutil::process::Process;
use serde_json::Value;
use std::env;
use std::time::Instant;
use uuid::Uuid;
use validator::Validate;



#[get("/serverless")]
async fn serverless(jsonbody: web::Json<RequestBody>) -> impl Responder {
    // Validation for the request json body
    if let Err(err) = jsonbody.validate() {
        println!("{}", err);
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
            println!("Error : {}", e);
            return HttpResponse::InternalServerError().json(resp);
        }
    };

    let call_data = json_response["result"]["input"].to_string();
    let user_address = json_response["result"]["from"].to_string();
    println!("\nUser address : {}", user_address);

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

    println!("Time taken to fetch data : {:?}", fetch_time);

    let decoded_calldata = match decode_call_data(&call_data) {
        Ok(data) => data,
        Err(e) => {
            println!("{}", e);
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
    println!("Free port :{}", &free_port);

    //Generating the js and capnp file
    let js_file = create_js_file(&decoded_calldata, &file_name, &workerd_runtime_path).await;

    match js_file {
        Ok(_) => {
            println!("JS file generated.")
        }
        Err(e) => {
            let resp = JsonResponse {
                status: "error".to_string(),
                message: "Error generating the JS file".to_string(),
                data: None,
            };
            println!("Error : {}", e);
            return HttpResponse::InternalServerError().json(resp);
        }
    };

    let capnp_file = create_capnp_file(&file_name, free_port, &workerd_runtime_path).await;

    match capnp_file {
        Ok(_) => {
            println!("Config file generated.")
        }
        Err(e) => {
            let resp = JsonResponse {
                status: "error".to_string(),
                message: "Error generating the configuration file".to_string(),
                data: None,
            };
            println!("Error : {}", e);
            return HttpResponse::InternalServerError().json(resp);
        }
    }

    let js_file_path = workerd_runtime_path.to_string() + &file_name.to_string() + ".js";
    let capnp_file_path = workerd_runtime_path.to_string() + &file_name.to_string() + ".capnp";

    //Run the workerd runtime with generated files

    let workerd_execution_start = Instant::now();
    let workerd = run_workerd_runtime(&file_name, &workerd_runtime_path).await;

    if workerd.is_err() {
        let _deleted_js_file = delete_file(&js_file_path);
        let _deleted_capnp_file = delete_file(&capnp_file_path);
        let workerd_error = workerd.err();
        println!("Error running the workerd runtime: {:?}", workerd_error);
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
            println!("{}",e);
            panic!("{}", e)
        }
    };

    // Wait for the port to bind
    if wait_for_port(free_port) {
        //Fetching workerd memory usage
        let workerd_process_pid = workerd_process.id();
        let process = Process::new(workerd_process_pid).expect("failed to get process info");
        let mem_info = process.memory_info().unwrap().rss();
        println!("Workerd memory usage: {}", mem_info);

        //Fetching the workerd response
        let workerd_respone = get_workerd_response(free_port).await.unwrap();

        //Terminating the workerd process once the response is fetched
        let kill_workerd_process = workerd_process.kill();

        //Fetching the workerd execution duration
        let workerd_execution_end = Instant::now();
        let workerd_execution_duration = workerd_execution_end
            .duration_since(workerd_execution_start)
            .as_millis()
            .to_string();
        println!("Workerd execution time: {}ms", workerd_execution_duration);

        match kill_workerd_process {
            Ok(_) => {
                println!("Workerd process {} terminated.", workerd_process.id())
            }
            Err(_) => {
                println!("Error terminating the process : {}", workerd_process.id())
            }
        }

        //Delete the generated file once the response is generated

        let _deleted_js_file = delete_file(&js_file_path);
        let _deleted_capnp_file = delete_file(&capnp_file_path);

        let resp = JsonResponse {
            status: "success".to_string(),
            message: "Response successfully generated".to_string(),
            data: Some(Value::String(workerd_respone)),
        };

        println!("Generated response");
        HttpResponse::Ok().json(resp)
    } else {

        let workerd_status = workerd_process.try_wait();
        match workerd_status {
            Ok(status) => {
                let error_status = status.unwrap().to_string();
                println!("Workerd execution error : {}",error_status);
                if error_status == "signal: 9 (SIGKILL)" {
                    let resp = JsonResponse {
                        status: "error".to_string(),
                        message: "Workerd ran out of memory".to_string(),
                        data: None,
                    };
                    return HttpResponse::InternalServerError().json(resp)
                }
            },
            Err(err) => panic!("Error fetching workerd exit status : {}",err)
        }

        let resp = JsonResponse {
            status: "error".to_string(),
            message: "Failed to bind to the workerd process".to_string(),
            data: None,
        };
        HttpResponse::InternalServerError().json(resp)
    }
}

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api")
        .service(serverless);

    conf.service(scope);
}
