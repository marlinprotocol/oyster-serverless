#[cfg(test)]
pub mod serverlesstest {

    use crate::model::AppState;
    use crate::{handler, serverless};
    use actix_web::{http, test, web, App};
    use dotenv::dotenv;
    use serde_json::json;
    use std::env;

    #[actix_web::test]
    async fn valid_input_test() {
        dotenv().ok();
        let cgroup_version: u8 = env::var("CGROUP_VERSION")
            .unwrap()
            .parse::<u8>()
            .expect("CGROUP VERSION must be a valid number ( Options: 1 or 2)");

        let cgroup_list = serverless::get_cgroup_list(cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version,
                }))
                .configure(handler::config),
        )
        .await;
        let valid_payload = json!({
            "code_id":"test",
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
    async fn code_id_not_provided() {
        dotenv().ok();
        let cgroup_version: u8 = env::var("CGROUP_VERSION")
            .unwrap()
            .parse::<u8>()
            .expect("CGROUP VERSION must be a valid number ( Options: 1 or 2)");

        let cgroup_list = serverless::get_cgroup_list(cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version,
                }))
                .configure(handler::config),
        )
        .await;

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
        let cgroup_version: u8 = env::var("CGROUP_VERSION")
            .unwrap()
            .parse::<u8>()
            .expect("CGROUP VERSION must be a valid number ( Options: 1 or 2)");

        let cgroup_list = serverless::get_cgroup_list(cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version,
                }))
                .configure(handler::config),
        )
        .await;

        let invalid_payload = json!({
            "code_id":"invalidjscode",
            "input": {
                "num": 100
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
    async fn invalid_payload_test() {
        dotenv().ok();
        let cgroup_version: u8 = env::var("CGROUP_VERSION")
            .unwrap()
            .parse::<u8>()
            .expect("CGROUP VERSION must be a valid number ( Options: 1 or 2)");

        let cgroup_list = serverless::get_cgroup_list(cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version,
                }))
                .configure(handler::config),
        )
        .await;

        let invalid_payload = json!({
            "code_id":"test"
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn response_timeout_test() {
        dotenv().ok();
        let cgroup_version: u8 = env::var("CGROUP_VERSION")
            .unwrap()
            .parse::<u8>()
            .expect("CGROUP VERSION must be a valid number ( Options: 1 or 2)");

        let cgroup_list = serverless::get_cgroup_list(cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version,
                }))
                .configure(handler::config),
        )
        .await;

        let invalid_payload = json!({
            "code_id":"timeouttest"
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::REQUEST_TIMEOUT);
    }
}
