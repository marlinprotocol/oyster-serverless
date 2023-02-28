use actix_web::{get,web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize};
use dotenv::dotenv;
use oyster::*;
use std::env;

#[derive(Deserialize)]
struct RequestBody {
    tx_hash: String,
}

#[get("/serverless")]
async fn serverless(jsonbody: web::Json<RequestBody>) -> impl Responder {

    let workerd_runtime_path =  env::var("RUNTIME_PATH").expect("RUNTIME_PATH must be a valid path");
    let tx_hash = &jsonbody.tx_hash;

    //Fetching the transaction data using the transaction hash and decoding the calldata
    let json_response = get_transaction_data(tx_hash).await.unwrap()["result"]["input"].to_string();
    let decoded_calldata = decode_call_data(&json_response);

    //Fetching a free port
    let free_port = get_free_port();

    //Generating the js and capnp file
    let js_file = create_js_file(decoded_calldata,tx_hash,workerd_runtime_path.to_string()).await;

    match js_file {
        Ok(_) => {println!("JS file generated.")},
        Err(e) =>{
            println!("Error : {}",e);
            return HttpResponse::InternalServerError().body("Error generating the JS file")
        }
    };

    let capnp_file = create_capnp_file(tx_hash,free_port,workerd_runtime_path.to_string()).await;
    
    match capnp_file {
        Ok(_) => {println!("Config file generated.")},
        Err(e) =>{
            println!("Error : {}",e);
            return HttpResponse::InternalServerError().body("Error generating the config file")
        }
    }

    let js_file_path = workerd_runtime_path.to_string()+&tx_hash.to_string()+".js";
    let capnp_file_path = workerd_runtime_path.to_string()+&tx_hash.to_string()+".capnp";

    //Run the workerd runtime with generated files

    let workerd = run_workerd_runtime(tx_hash,workerd_runtime_path.to_string()).await;
    
    if workerd.is_err() == true {
        let _deleted_js_file = delete_file(&js_file_path);
        let _deleted_capnp_file = delete_file(&capnp_file_path);
        let workerd_error = workerd.err();
        println!("Error running the workerd runtime: {:?}",workerd_error);
        return HttpResponse::InternalServerError().body("Error running the workerd runtime")
    }

    let mut workerd_process = match workerd {
        Ok(data) => data,
        Err(e) =>{
            println!("Error : {}",e);
            return HttpResponse::InternalServerError().body("Error running the workerd runtime")
        }
    };

    // Wait for the port to bind
    if wait_for_port(free_port){
        let workerd_respone = get_workerd_response(free_port).await.unwrap();
        let _kill_workerd_process = workerd_process.kill();
        
        //Delete the generated file once the response is generated

        let _deleted_js_file = delete_file(&js_file_path);
        let _deleted_capnp_file = delete_file(&capnp_file_path);
    
        HttpResponse::Ok().body(workerd_respone)
    }else{
        HttpResponse::InternalServerError().body("Error in fetching resposne from the workerd runtime")
    }

}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let port:u16 = env::var("PORT").unwrap().parse::<u16>().expect("PORT must be a valid number");
    let server = HttpServer::new(|| {
        App::new()
            .service(serverless)
    })
    .bind(("0.0.0.0",port))?
    .run();
    println!("Server started on port {}",port);
    server.await
}