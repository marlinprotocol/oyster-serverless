use crate::error::ServerlessError;
use crate::file_manager;
use crate::{cgroups, model::AppState, workerd};

use actix_web::http::{header, StatusCode};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use anyhow::{anyhow, Context};
use std::io::{BufRead, BufReader};
use std::sync::atomic::Ordering;
use std::time::Duration;
// use std::time::Instant;
use tokio::time::timeout;

pub async fn serverless(
    body: web::Bytes,
    appstate: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // check if the server is draining
    // IMPORTANT: we use Relaxed ordering here since we do not need to synchronize any memory
    // not even with reads/writes to the same atomic (we just serve a few more requests at worst)
    // be very careful adding more operations associated with the draining state
    if !appstate.running.load(Ordering::Relaxed) {
        return HttpResponse::Gone().body("worker unregistered");
    }

    // get the host header value
    let host_header = req
        .headers()
        .get(header::HOST)
        .context("could not find Host header")
        .and_then(|x| x.to_str().context("could not parse Host header"));
    if let Err(err) = host_header {
        return HttpResponse::BadRequest().body(format!("{:?}", err));
    }
    let host_header = host_header.unwrap();

    // get tx hash by splitting, will always have at least one element
    let tx_hash = host_header.split('.').next().unwrap().to_owned();

    // handle unregister here
    if tx_hash == "unregister" {
        // IMPORTANT: we use Relaxed ordering here since we do not need to synchronize any memory
        // not even with reads/writes to the same atomic (we just serve a few more requests at worst)
        // be very careful adding more operations associated with the draining state
        appstate.running.store(false, Ordering::Relaxed);

        return HttpResponse::Ok()
            .status(StatusCode::OK)
            .body("successfully set server in draining state");
    }

    let workerd_runtime_path = &appstate.runtime_path;
    let workerd_cache_path = &appstate.cache_path;

    let slug = &hex::encode(rand::random::<u32>().to_ne_bytes());

    // decode base32 into hex
    let tx_hash = data_encoding::BASE32_NOPAD.decode(tx_hash.to_uppercase().as_bytes());
    if let Err(err) = tx_hash {
        return HttpResponse::BadRequest().body(format!("invalid tx hash encoding: {:?}", err));
    }
    let tx_hash = tx_hash.unwrap();
    let tx_hash = &("0x".to_owned() + &data_encoding::HEXLOWER.encode(&tx_hash));

    // create code file
    if let Err(err) = file_manager::create_code_file(
        tx_hash,
        slug,
        workerd_runtime_path,
        &workerd_cache_path,
        &appstate.rpc,
        &appstate.contract,
    )
    .await
    {
        use ServerlessError::*;
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
            CodeFileCreate(_) | CodeFileCount(_) | UpdateModifiedTime(_) => {
                HttpResponse::InternalServerError().body(format!(
                    "{:?}",
                    anyhow!(err).context("failed to create code file")
                ))
            }
            _ => HttpResponse::InternalServerError().body(format!(
                "{:?}",
                anyhow!(err).context("unexpected error while trying to create code file")
            )),
        };
    }

    // let execution_timer_start = Instant::now();

    // reserve cgroup
    let cgroup = appstate.cgroups.lock().unwrap().reserve();
    if let Err(err) = cgroup {
        // cleanup
        file_manager::cleanup_code_file(tx_hash, slug, workerd_runtime_path)
            .await
            .context("CRITICAL: failed to clean up code file")
            .unwrap_or_else(|err| println!("{err:?}"));

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
        appstate.cgroups.lock().unwrap().release(cgroup);
        file_manager::cleanup_code_file(tx_hash, slug, workerd_runtime_path)
            .await
            .context("CRITICAL: failed to clean up code file")
            .unwrap_or_else(|err| println!("{err:?}"));

        return match err {
            ServerlessError::BadPort(_) => {
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
    if let Err(err) =
        file_manager::create_config_file(tx_hash, slug, workerd_runtime_path, port).await
    {
        // cleanup
        appstate.cgroups.lock().unwrap().release(cgroup);
        file_manager::cleanup_code_file(tx_hash, slug, workerd_runtime_path)
            .await
            .context("CRITICAL: failed to clean up code file")
            .unwrap_or_else(|err| println!("{err:?}"));

        use ServerlessError::*;
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
        file_manager::cleanup_config_file(tx_hash, slug, workerd_runtime_path)
            .await
            .context("CRITICAL: failed to clean up config file")
            .unwrap_or_else(|err| println!("{err:?}"));
        appstate.cgroups.lock().unwrap().release(cgroup);
        file_manager::cleanup_code_file(tx_hash, slug, workerd_runtime_path)
            .await
            .context("CRITICAL: failed to clean up code file")
            .unwrap_or_else(|err| println!("{err:?}"));

        return HttpResponse::BadRequest().body(format!(
            "{:?}",
            anyhow!(err).context("failed to execute worker")
        ));
    }
    let mut child = child.unwrap();

    // wait for worker to be available
    let res = workerd::wait_for_port(port).await;

    if !res {
        // cleanup
        child
            .kill()
            .context("CRITICAL: failed to kill worker {cgroup}")
            .unwrap_or_else(|err| println!("{err:?}"));
        file_manager::cleanup_config_file(tx_hash, slug, workerd_runtime_path)
            .await
            .context("CRITICAL: failed to clean up config file")
            .unwrap_or_else(|err| println!("{err:?}"));
        appstate.cgroups.lock().unwrap().release(cgroup);
        file_manager::cleanup_code_file(tx_hash, slug, workerd_runtime_path)
            .await
            .context("CRITICAL: failed to clean up code file")
            .unwrap_or_else(|err| println!("{err:?}"));

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
    let host_header = host_header.to_owned();
    let response = timeout(
        Duration::from_secs(5),
        workerd::get_workerd_response(port, req, body, &appstate.signer, &host_header),
    )
    .await;

    // cleanup
    child
        .kill()
        .context("CRITICAL: failed to kill worker {cgroup}")
        .unwrap_or_else(|err| println!("{err:?}"));
    file_manager::cleanup_config_file(tx_hash, slug, workerd_runtime_path)
        .await
        .context("CRITICAL: failed to clean up config file")
        .unwrap_or_else(|err| println!("{err:?}"));
    appstate.cgroups.lock().unwrap().release(cgroup);
    file_manager::cleanup_code_file(tx_hash, slug, workerd_runtime_path)
        .await
        .context("CRITICAL: failed to clean up code file")
        .unwrap_or_else(|err| println!("{err:?}"));

    if let Err(err) = response {
        return HttpResponse::RequestTimeout()
            .body(format!("{:?}", anyhow!(err).context("worker timed out")));
    }
    let response = response.unwrap();

    if let Err(err) = response {
        return HttpResponse::InternalServerError().body(format!(
            "{:?}",
            anyhow!(err).context("failed to get a response")
        ));
    }
    let response = response.unwrap();

    // let execution_timer_end = Instant::now();
    // let execution_time = execution_timer_end
    //     .duration_since(execution_timer_start)
    //     .as_millis()
    //     .to_string();
    // println!("Execution time: {}ms", execution_time);

    return response;
}
