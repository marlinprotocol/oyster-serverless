// TODO: tests have to be run one by one currently
// I *think* it is because parallel actix services all get the same cgroup list
// which means they all get the same port mappings and might collide
// But it might also be hidden concurrency issues, investigate and fix

#[cfg(test)]
pub mod serverlesstest {
    use crate::cgroups::Cgroups;
    use crate::handler;
    use crate::model::AppState;
    use actix_web::{body::MessageBody, http, test, web, App};
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    #[actix_web::test]
    async fn valid_input_test() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroups: Cgroups::new().unwrap().into(),
                    running: AtomicBool::new(true),
                    runtime_path: "./runtime/".to_owned(),
                }))
                .default_service(web::to(handler::serverless)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.serverless.dev",
            ))
            .set_json(&json!({
                "num": 10
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.into_body().try_into_bytes().unwrap(), "2,5");

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.serverless.dev",
            ))
            .set_json(&json!({
                "num": 20
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.into_body().try_into_bytes().unwrap(), "2,2,5");

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.serverless.dev",
            ))
            .set_json(&json!({
                "num": 600
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.into_body().try_into_bytes().unwrap(), "2,2,2,3,5,5");
    }

    #[actix_web::test]
    async fn valid_input_different_uri_test() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroups: Cgroups::new().unwrap().into(),
                    running: AtomicBool::new(true),
                    runtime_path: "./runtime/".to_owned(),
                }))
                .default_service(web::to(handler::serverless)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/serverless")
            .append_header((
                "Host",
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.serverless.dev",
            ))
            .set_json(&json!({
                "num": 10
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.into_body().try_into_bytes().unwrap(), "2,5");

        let req = test::TestRequest::post()
            .uri("/serverless")
            .append_header((
                "Host",
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.serverless.dev",
            ))
            .set_json(&json!({
                "num": 20
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.into_body().try_into_bytes().unwrap(), "2,2,5");

        let req = test::TestRequest::post()
            .uri("/serverless")
            .append_header((
                "Host",
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.serverless.dev",
            ))
            .set_json(&json!({
                "num": 600
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.into_body().try_into_bytes().unwrap(), "2,2,2,3,5,5");
    }

    #[actix_web::test]
    async fn valid_input_different_method_test() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroups: Cgroups::new().unwrap().into(),
                    running: AtomicBool::new(true),
                    runtime_path: "./runtime/".to_owned(),
                }))
                .default_service(web::to(handler::serverless)),
        )
        .await;
        let valid_payload = json!({
            "num": 10
        });

        let req = test::TestRequest::get()
            .uri("/")
            .append_header((
                "Host",
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.serverless.dev",
            ))
            .set_json(&valid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn interacting_with_wrong_smartcontract() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroups: Cgroups::new().unwrap().into(),
                    running: AtomicBool::new(true),
                    runtime_path: "./runtime/".to_owned(),
                }))
                .default_service(web::to(handler::serverless)),
        )
        .await;

        let invalid_payload = json!({
            "num": 10
        });

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                "0x37b0b2d9dd58d9130781fc914da456c16ec403010e8d4c27b0ea4657a24c8546.serverless.dev",
            ))
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn invalid_txhash() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroups: Cgroups::new().unwrap().into(),
                    running: AtomicBool::new(true),
                    runtime_path: "./runtime/".to_owned(),
                }))
                .default_service(web::to(handler::serverless)),
        )
        .await;

        let invalid_payload = json!({
            "num": 10
        });

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                "0x37b0b2d9dd58d9130781fc914da456c16ec403010e8d4c27b0ea4657a24c8546.serverless.dev",
            ))
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn txhash_not_provided() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroups: Cgroups::new().unwrap().into(),
                    running: AtomicBool::new(true),
                    runtime_path: "./runtime/".to_owned(),
                }))
                .default_service(web::to(handler::serverless)),
        )
        .await;

        let invalid_payload = json!({});

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn invalid_js_code_in_calldata() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroups: Cgroups::new().unwrap().into(),
                    running: AtomicBool::new(true),
                    runtime_path: "./runtime/".to_owned(),
                }))
                .default_service(web::to(handler::serverless)),
        )
        .await;

        let invalid_payload = json!({
            "num": 100
        });

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                "0x3d2deb53d077f88b40cdf3a81ce3cac6367fddce22f1f131e322e7463ce34f8f.serverless.dev",
            ))
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn invalid_payload_test() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroups: Cgroups::new().unwrap().into(),
                    running: AtomicBool::new(true),
                    runtime_path: "./runtime/".to_owned(),
                }))
                .default_service(web::to(handler::serverless)),
        )
        .await;

        let invalid_payload = json!({});

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113.serverless.dev",
            ))
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn response_timeout_test() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    cgroups: Cgroups::new().unwrap().into(),
                    running: AtomicBool::new(true),
                    runtime_path: "./runtime/".to_owned(),
                }))
                .default_service(web::to(handler::serverless)),
        )
        .await;

        let invalid_payload = json!({});

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                "0xf17fb991c648e8bdc93f2dcfccc25c98774084ee4ae398f0b289e698b9992303.serverless.dev",
            ))
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::REQUEST_TIMEOUT);
    }
}
