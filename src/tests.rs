// TODO: tests have to be run one by one currently
// I *think* it is because parallel actix services all get the same cgroup list
// which means they all get the same port mappings and might collide
// But it might also be hidden concurrency issues, investigate and fix

#[cfg(test)]
pub mod serverlesstest {
    use crate::cgroups::Cgroups;
    use crate::handler;
    use crate::model::AppState;
    use crate::BillingContract;
    use actix_web::{
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        error::Error,
        http, test, web, App,
    };
    use ethers::providers::{Http, Provider};
    use ethers::types::Address;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::Duration;

    fn new_app() -> App<
        impl ServiceFactory<
            ServiceRequest,
            Response = ServiceResponse<impl MessageBody + std::fmt::Debug>,
            Config = (),
            InitError = (),
            Error = Error,
        >,
    > {
        let rpc = String::from("https://sepolia-rollup.arbitrum.io/rpc");
        let billing_contract_add = String::new(); // TODO: ADD BILLING CONTRACT FOR TESTS

        let rpc_provider = Provider::<Http>::try_from(&rpc)
            .unwrap()
            .interval(Duration::from_millis(1000));
        let billing_contract = BillingContract::new(
            billing_contract_add.parse::<Address>().unwrap(),
            Arc::new(rpc_provider),
        );

        App::new()
            .app_data(web::Data::new(AppState {
                cgroups: Cgroups::new().unwrap().into(),
                running: AtomicBool::new(true),
                runtime_path: "./runtime/".to_owned(),
                rpc: rpc,
                contract: "0x44fe06d2940b8782a0a9a9ffd09c65852c0156b1".to_owned(),
                signer: k256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng),
                billing_contract: billing_contract,
                execution_costs: HashMap::new().into(),
                last_bill_claim: (None, None).into(),
            }))
            .default_service(web::to(handler::serverless))
    }

    #[actix_web::test]
    async fn valid_input_test() {
        let app = test::init_service(new_app()).await;

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
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
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
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
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
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
    async fn valid_input_lowercase_test() {
        let app = test::init_service(new_app()).await;

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "srulw2uoqxwrdyuszdfmbqkttx3jdsgy5rropw72t4n5p5ie4rxa.oyster.run",
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
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "srulw2uoqxwrdyuszdfmbqkttx3jdsgy5rropw72t4n5p5ie4rxa.oyster.run",
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
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "srulw2uoqxwrdyuszdfmbqkttx3jdsgy5rropw72t4n5p5ie4rxa.oyster.run",
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
        let app = test::init_service(new_app()).await;

        let req = test::TestRequest::post()
            .uri("/serverless")
            .append_header((
                "Host",
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
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
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
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
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
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
        let app = test::init_service(new_app()).await;

        let req = test::TestRequest::get()
            .uri("/")
            .append_header((
                "Host",
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
            ))
            .set_json(&json!({
                "num": 10
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.into_body().try_into_bytes().unwrap(), "2,5");

        let req = test::TestRequest::get()
            .uri("/")
            .append_header((
                "Host",
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
            ))
            .set_json(&json!({
                "num": 20
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.into_body().try_into_bytes().unwrap(), "2,2,5");

        let req = test::TestRequest::get()
            .uri("/")
            .append_header((
                "Host",
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
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
    async fn interacting_with_wrong_smartcontract() {
        let app = test::init_service(new_app()).await;

        let payload = json!({
            "num": 10
        });

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                // "0xfed8ab36cc27831836f6dcb7291049158b4d8df31c0ffb05a3d36ba6555e29d7.oyster.run",
                "73MKWNWME6BRQNXW3S3SSECJCWFU3DPTDQH7WBND2NV2MVK6FHLQ.oyster.run",
            ))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.into_body().try_into_bytes().unwrap(),
            "failed to create code file\n\nCaused by:\n    to address 0x0784e2d4551905f66269b133aa4f43fe3d23b707 does not match expected 0x44fe06d2940b8782a0a9a9ffd09c65852c0156b1"
        );
    }

    #[actix_web::test]
    async fn invalid_txhash() {
        let app = test::init_service(new_app()).await;

        let payload = json!({
            "num": 10
        });

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                // "0x37b0b2d9dd58d9130781fc914da456c16ec403010e8d4c27b0ea4657a24c8546.oyster.run",
                "G6YLFWO5LDMRGB4B7SIU3JCWYFXMIAYBB2GUYJ5Q5JDFPISMQVDA.oyster.run",
            ))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.into_body().try_into_bytes().unwrap(),
            "failed to create code file\n\nCaused by:\n    tx not found"
        );
    }

    #[actix_web::test]
    async fn txhash_not_provided() {
        let app = test::init_service(new_app()).await;

        let payload = json!({});

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.into_body().try_into_bytes().unwrap(),
            "could not find Host header"
        );
    }

    #[actix_web::test]
    async fn invalid_js_code_in_calldata() {
        let app = test::init_service(new_app()).await;

        let payload = json!({
            "num": 100
        });

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                // "0x96179f60fd7917c04ad9da6dd64690a1a960f39b50029d07919bf2628f5e7fe5.oyster.run",
                "SYLZ6YH5PEL4ASWZ3JW5MRUQUGUWB443KABJ2B4RTPZGFD26P7SQ.oyster.run",
            ))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.into_body().try_into_bytes().unwrap(),
            "syntax error in the code: service main: Uncaught SyntaxError: Unexpected token 'export'\n  at main:1:1"
        );
    }

    #[actix_web::test]
    async fn invalid_payload_test() {
        let app = test::init_service(new_app()).await;

        let invalid_payload = json!({});

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RXA.oyster.run",
            ))
            .set_json(&invalid_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.into_body().try_into_bytes().unwrap(),
            "Please provide a valid integer as input in the format{'num':10}"
        );
    }

    #[actix_web::test]
    async fn response_timeout_test() {
        let app = test::init_service(new_app()).await;

        let payload = json!({});

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                // "0x9c641b535e5586200d0f2fd81f05a39436c0d9dd35530e9fb3ca18352c3ba111.oyster.run",
                "TNJKQVX65RLEDOOL5LW5CWQ2RUHF2LSQQIBXXXNFSVMRLLFOLKIQ.oyster.run",
            ))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::REQUEST_TIMEOUT);
        assert_eq!(
            resp.into_body().try_into_bytes().unwrap(),
            "worker timed out\n\nCaused by:\n    deadline has elapsed"
        );
    }

    #[actix_web::test]
    async fn invalid_tx_hash_encoding_test() {
        let app = test::init_service(new_app()).await;

        let payload = json!({
            "num": 10,
        });

        let req = test::TestRequest::post()
            .uri("/")
            .append_header((
                "Host",
                // "0x9468bb6a8e85ed11e292c8cac0c1539df691c8d8ec62e7dbfa9f1bd7f504e46e.oyster.run",
                "SRULW2UOQXWRDYUSZDFMBQKTTX3JDSGY5RROPW72T4N5P5IE4RX0.oyster.run",
            ))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.into_body().try_into_bytes().unwrap(),
            "invalid tx hash encoding: DecodeError { position: 51, kind: Symbol }"
        );
    }
}
