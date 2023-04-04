mod handler;
mod model;
mod response;
mod serverless;

use crate::model::AppState;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;

use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log::info!("Make sure you have set up cgroups on your system by following the instructions in the readme file.");

    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let cgroup_list = serverless::get_cgroup_list();
    if cgroup_list.is_empty() {
        log::error!("No cgroups found. Please create cgroups and try again.");
        std::process::exit(1);
    }

    let port: u16 = env::var("PORT")
        .unwrap()
        .parse::<u16>()
        .expect("PORT must be a valid number");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                cgroup_list: cgroup_list.clone(),
            }))
            .configure(handler::config)
    })
    .bind(("0.0.0.0", port))
    .unwrap_or_else(|_| panic!("Can not bind to {}", &port))
    .run();

    log::info!("Server started on port {}", port);

    server.await
}

#[cfg(test)]
pub mod serverlesstest {

    use super::*;
    use actix_web::{http, test, App};
    use serde_json::json;

    #[actix_web::test]
    async fn valid_input_test() {
        dotenv().ok();
        let app = test::init_service(App::new().configure(handler::config)).await;
        let valid_payload = json!({
            "tx_hash": "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113",
            "input": {
                "num": 10
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&valid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn interacting_with_wrong_smartcontract() {
        dotenv().ok();
        let app = test::init_service(App::new().configure(handler::config)).await;

        let invalid_payload = json!({
            "tx_hash": "0x37b0b2d9dd58d9130781fc914da456c16ec403010e8d4c27b0ea4657a24c8546",
            "input": {
                "num": 10
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn invalid_txhash() {
        dotenv().ok();
        let app = test::init_service(App::new().configure(handler::config)).await;

        let invalid_payload = json!({
            "tx_hash": "0x37b0b2d9dd58d9130781fc914da456c16ec403010e8d4c27b0ea4657a24c85",
            "input": {
                "num": 10
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn txhash_not_provided() {
        dotenv().ok();
        let app = test::init_service(App::new().configure(handler::config)).await;

        let invalid_payload = json!({});

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn invalid_js_code_in_calldata() {
        dotenv().ok();
        let app = test::init_service(App::new().configure(handler::config)).await;

        let invalid_payload = json!({
            "tx_hash": "0x898ebb6887cba44eb53601af2ace75ef1bfadc78ebfeb55ced33d9b83f8d8d4e",
            "input": {
                "num": 10
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn invalid_payload_test() {
        dotenv().ok();
        let app = test::init_service(App::new().configure(handler::config)).await;

        let invalid_payload = json!({
            "tx_hash": "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113"
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
    }
}
