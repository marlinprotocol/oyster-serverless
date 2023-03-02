use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use oyster::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
struct RequestBody {
    #[validate(length(min = 1), required)]
    tx_hash: Option<String>,
}

#[derive(Serialize)]
struct JsonResponse {
    status: String,
    message: String,
    data: Option<Value>,
}

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

    //Fetching the transaction data using the transaction hash and decoding the calldata
    let json_response = match get_transaction_data(tx_hash).await {
        Ok(data) => data["result"]["input"].to_string(),
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

    if json_response == "null" {
        let resp = JsonResponse {
            status: "error".to_string(),
            message: "Error fetching the call data, make sure a valid tx_hash is provided"
                .to_string(),
            data: None,
        };
        return HttpResponse::BadRequest().json(resp);
    }

    let decoded_calldata = match decode_call_data(&json_response) {
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

    //Generating the js and capnp file
    let js_file = create_js_file(&decoded_calldata, &file_name, &workerd_runtime_path).await;

    match js_file {
        Ok(_) => {
            println!("\nJS file generated.")
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
            panic!("{}", e)
        }
    };

    // Wait for the port to bind
    if wait_for_port(free_port) {
        let workerd_respone = get_workerd_response(free_port).await.unwrap();
        let kill_workerd_process = workerd_process.kill();

        match kill_workerd_process {
            Ok(_) => {
                println!("Process {} terminated", workerd_process.id())
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
        let resp = JsonResponse {
            status: "error".to_string(),
            message: "Failed to bind to the workerd process".to_string(),
            data: None,
        };
        HttpResponse::InternalServerError().json(resp)
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let port: u16 = env::var("PORT")
        .unwrap()
        .parse::<u16>()
        .expect("PORT must be a valid number");
    let server = HttpServer::new(|| App::new().service(serverless))
        .bind(("0.0.0.0", port))?
        .run();
    println!("Server started on port {}", port);
    server.await
}
