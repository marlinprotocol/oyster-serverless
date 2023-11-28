use crate::{
    cgroups,
    model::{AppState, RequestBody},
    response::response,
    serverless::*,
    workerd,
};

use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpResponse, Responder};
use anyhow::{anyhow, Context};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::sync::atomic::Ordering;
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
    // check if the server is draining
    // IMPORTANT: we use Relaxed ordering here since we do not need to synchronize any memory
    // not even with reads/writes to the same atomic (we just serve a few more requests at worst)
    // be very careful adding more operations associated with the draining state
    if !appstate.running.load(Ordering::Relaxed) {
        return HttpResponse::Gone().body("worker unregistered");
    }

    // validate request body
    if let Err(err) = jsonbody.validate() {
        return HttpResponse::BadRequest()
            .body(format!("{:?}", anyhow!(err).context("invalid payload")));
    }

    let tx_hash = &jsonbody.tx_hash;
    let slug = &hex::encode(rand::random::<u32>().to_ne_bytes());
    let workerd_runtime_path = &appstate.runtime_path;

    // create code file
    if let Err(err) = workerd::create_code_file(tx_hash, slug, workerd_runtime_path).await {
        use workerd::ServerlessError::*;
        return match err {
            CalldataRetrieve(_)
            | TxNotFound
            | InvalidTxToType
            | InvalidTxToValue(_, _)
            | InvalidTxCalldataType
            | BadCalldata(_) => HttpResponse::BadRequest().body(format!(
                "{:?}",
                anyhow!(err).context("failed to create code file")
            )),
            CodeFileCreate(_) => HttpResponse::InternalServerError().body(format!(
                "{:?}",
                anyhow!(err).context("failed to create code file")
            )),
            _ => HttpResponse::InternalServerError().body(format!(
                "{:?}",
                anyhow!(err).context("unexpected error while trying to create code file")
            )),
        };
    }

    let execution_timer_start = Instant::now();

    // reserve cgroup
    let cgroup = appstate.cgroups.reserve();
    if let Err(err) = cgroup {
        // cleanup
        workerd::cleanup_code_file(tx_hash, slug, workerd_runtime_path).await;

        return match err {
            cgroups::CgroupsError::NoFree => {
                return HttpResponse::TooManyRequests().body(format!(
                    "{:?}",
                    anyhow!("no free cgroup available to run request")
                ))
            }
            _ => HttpResponse::InternalServerError().body(format!(
                "{:?}",
                anyhow!(err).context("unexpected error while trying to reserve cgroup")
            )),
        };
    }
    let cgroup = cgroup.unwrap();

    // get port for cgroup
    let port = workerd::get_port(&cgroup);
    if let Err(err) = port {
        // cleanup
        appstate.cgroups.release(cgroup);
        workerd::cleanup_code_file(tx_hash, slug, workerd_runtime_path).await;

        return match err {
            workerd::ServerlessError::BadPort(_) => {
                return HttpResponse::InternalServerError().body(format!(
                    "{:?}",
                    anyhow!(err).context("failed to get port for cgroup")
                ))
            }
            _ => HttpResponse::InternalServerError().body(format!(
                "{:?}",
                anyhow!(err).context("unexpected error while trying to get port for cgroup")
            )),
        };
    }
    let port = port.unwrap();

    // create config file
    if let Err(err) = workerd::create_config_file(tx_hash, slug, workerd_runtime_path, port).await {
        // cleanup
        appstate.cgroups.release(cgroup);
        workerd::cleanup_code_file(tx_hash, slug, workerd_runtime_path).await;

        use workerd::ServerlessError::*;
        return match err {
            CalldataRetrieve(_)
            | TxNotFound
            | InvalidTxToType
            | InvalidTxToValue(_, _)
            | InvalidTxCalldataType
            | BadCalldata(_) => HttpResponse::BadRequest().body(format!(
                "{:?}",
                anyhow!(err).context("failed to create code file")
            )),
            CodeFileCreate(_) => HttpResponse::InternalServerError().body(format!(
                "{:?}",
                anyhow!(err).context("failed to create code file")
            )),
            _ => HttpResponse::InternalServerError().body(format!(
                "{:?}",
                anyhow!(err).context("unexpected error while trying to create code file")
            )),
        };
    }

    // start worker
    let child = workerd::execute(tx_hash, slug, workerd_runtime_path, &cgroup).await;
    if let Err(err) = child {
        // cleanup
        workerd::cleanup_config_file(tx_hash, slug, workerd_runtime_path).await;
        appstate.cgroups.release(cgroup);
        workerd::cleanup_code_file(tx_hash, slug, workerd_runtime_path).await;

        return HttpResponse::BadRequest().body(format!(
            "{:?}",
            anyhow!(err).context("failed to execute worker")
        ));
    }
    let child = child.unwrap();

    // wait for worker to be available
    let res = workerd::wait_for_port(port).await;

    if !res {
        // cleanup
        child.kill();
        workerd::cleanup_config_file(tx_hash, slug, workerd_runtime_path).await;
        appstate.cgroups.release(cgroup);
        workerd::cleanup_code_file(tx_hash, slug, workerd_runtime_path).await;

        let stderr = child.stderr.take().unwrap();
        let reader = BufReader::new(stderr);
        let stderr_lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
        let stderr_output = stderr_lines.join("\n");

        if stderr_output != "" && stderr_output.contains("SyntaxError") {
            return HttpResponse::BadRequest()
                .body(format!("syntax error in the code: {stderr_output}"));
        }

        return HttpResponse::InternalServerError()
            .body(format!("failed to execute worker: {stderr_output}"));
    }

    // worker is ready, make the request

    // Wait for the port to bind
    if res {
        //Fetching the workerd response
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

#[get("/unregister")]
async fn unregister(appstate: web::Data<AppState>) -> impl Responder {
    // IMPORTANT: we use Relaxed ordering here since we do not need to synchronize any memory
    // not even with reads/writes to the same atomic (we just serve a few more requests at worst)
    // be very careful adding more operations associated with the draining state
    appstate.running.store(false, Ordering::Relaxed);

    return HttpResponse::Ok()
        .status(StatusCode::OK)
        .body("successfully set server in draining state");
}

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api").service(serverless).service(unregister);
    conf.service(scope);
}
