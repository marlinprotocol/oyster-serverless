#[cfg(test)]
pub mod serverlesstest {

    use crate::model::AppState;
    use crate::{handler, serverless};
    use actix_web::{http, test, web, App};
    use clap::Parser;
    use reqwest::header;
    use serde_json::json;
    use std::sync::Mutex;

    use crate::Args;

    #[actix_web::test]
    async fn valid_input_test() {
        let cli = Args::parse();
        let cgroup_list = serverless::get_cgroup_list(cli.cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version: cli.cgroup_version,
                    running: Mutex::new(true),
                    runtime_path: cli.runtime_path,
                }))
                .configure(handler::config),
        )
        .await;
        let valid_payload = json!({
            "input": {
                "num": 10
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&valid_payload)
            .append_header((
                header::HOST,
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.localhost",
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn interacting_with_wrong_smartcontract() {
        let cli = Args::parse();
        let cgroup_list = serverless::get_cgroup_list(cli.cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version: cli.cgroup_version,
                    running: Mutex::new(true),
                    runtime_path: cli.runtime_path,
                }))
                .configure(handler::config),
        )
        .await;

        let valid_payload = json!({
            "input": {
                "num": 10
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&valid_payload)
            .append_header((
                header::HOST,
                "0x37b0b2d9dd58d9130781fc914da456c16ec403010e8d4c27b0ea4657a24c8546.localhost",
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn invalid_txhash() {
        let cli = Args::parse();
        let cgroup_list = serverless::get_cgroup_list(cli.cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version: cli.cgroup_version,
                    running: Mutex::new(true),
                    runtime_path: cli.runtime_path,
                }))
                .configure(handler::config),
        )
        .await;

        let valid_payload = json!({
            "input": {
                "num": 10
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&valid_payload)
            .append_header((header::HOST, "not-a-hash.localhost"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn txhash_not_provided() {
        let cli = Args::parse();
        let cgroup_list = serverless::get_cgroup_list(cli.cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version: cli.cgroup_version,
                    running: Mutex::new(true),
                    runtime_path: cli.runtime_path,
                }))
                .configure(handler::config),
        )
        .await;

        let valid_payload = json!({
            "input": {
                "num": 10
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&valid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn invalid_js_code_in_calldata() {
        let cli = Args::parse();
        let cgroup_list = serverless::get_cgroup_list(cli.cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version: cli.cgroup_version,
                    running: Mutex::new(true),
                    runtime_path: cli.runtime_path,
                }))
                .configure(handler::config),
        )
        .await;

        let valid_payload = json!({
            "input": {
                "num": 100
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&valid_payload)
            .append_header((
                header::HOST,
                "0x3d2deb53d077f88b40cdf3a81ce3cac6367fddce22f1f131e322e7463ce34f8f.localhost",
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn invalid_payload_test() {
        let cli = Args::parse();
        let cgroup_list = serverless::get_cgroup_list(cli.cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version: cli.cgroup_version,
                    running: Mutex::new(true),
                    runtime_path: cli.runtime_path,
                }))
                .configure(handler::config),
        )
        .await;

        let invalid_payload = json!({});

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&invalid_payload)
            .append_header((
                header::HOST,
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.localhost",
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn response_timeout_test() {
        let cli = Args::parse();
        let cgroup_list = serverless::get_cgroup_list(cli.cgroup_version).unwrap();
        if cgroup_list.is_empty() {
            log::error!("No cgroups found. Make sure you have set up cgroups on your system by following the instructions in the readme file.");
            std::process::exit(1);
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroup_list: cgroup_list.clone(),
                    cgroup_version: cli.cgroup_version,
                    running: Mutex::new(true),
                    runtime_path: cli.runtime_path,
                }))
                .configure(handler::config),
        )
        .await;

        let invalid_payload = json!({});

        let req = test::TestRequest::post()
            .uri("/api/serverless")
            .set_json(&invalid_payload)
            .append_header((
                header::HOST,
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.localhost",
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::REQUEST_TIMEOUT);
    }
}
