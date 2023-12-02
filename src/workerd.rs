use std::process::Child;
use std::time::{Duration, Instant};

use thiserror::Error;

use actix_web::{HttpRequest, HttpResponse};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::sleep;

use crate::cgroups::{Cgroups, CgroupsError};

#[derive(Error, Debug)]
pub enum ServerlessError {
    #[error("failed to retrieve calldata")]
    CalldataRetrieve(#[from] reqwest::Error),
    #[error("tx not found")]
    TxNotFound,
    #[error("to field of transaction is not an address")]
    InvalidTxToType,
    #[error("to address {0} does not match expected {1}")]
    InvalidTxToValue(String, &'static str),
    #[error("calldata field of transaction is not a string")]
    InvalidTxCalldataType,
    #[error("calldata is not a valid hex string")]
    BadCalldata(#[from] hex::FromHexError),
    #[error("failed to create code file")]
    CodeFileCreate(#[source] tokio::io::Error),
    #[error("failed to create config file")]
    ConfigFileCreate(#[source] tokio::io::Error),
    #[error("failed to execute workerd")]
    Execute(#[from] CgroupsError),
    #[error("failed to terminate workerd")]
    Terminate(#[source] tokio::io::Error),
    #[error("failed to delete code file")]
    CodeFileDelete(#[source] tokio::io::Error),
    #[error("failed to delete config file")]
    ConfigFileDelete(#[source] tokio::io::Error),
    #[error("failed to retrieve port from cgroup")]
    BadPort(#[source] std::num::ParseIntError),
}

async fn get_transaction_data(tx_hash: &str, rpc: &str) -> Result<Value, reqwest::Error> {
    let client = Client::new();
    let method = "eth_getTransactionByHash";
    let params = json!([&tx_hash]);
    let id = 1;

    let request = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id,
    });

    let response = client.post(rpc).json(&request).send().await?;
    let json_response = response.json::<Value>().await?;

    Ok(json_response)
}

pub async fn create_code_file(
    tx_hash: &str,
    slug: &str,
    workerd_runtime_path: &str,
    rpc: &str,
) -> Result<(), ServerlessError> {
    // get tx data
    let mut tx_data = match get_transaction_data(tx_hash, rpc).await?["result"].take() {
        Value::Null => Err(ServerlessError::TxNotFound),
        other => Ok(other),
    }?;

    // get contract address
    let contract_address = match tx_data["to"].take() {
        Value::String(value) => Ok(value),
        _ => Err(ServerlessError::InvalidTxToType),
    }?;

    // check contract address matches expected
    if contract_address != "0x30694a76d737211a908d0dd672f47e1d29fbfb02" {
        return Err(ServerlessError::InvalidTxToValue(
            contract_address,
            "0x30694a76d737211a908d0dd672f47e1d29fbfb02",
        ));
    }

    // get calldata
    let calldata = match tx_data["input"].take() {
        Value::String(calldata) => Ok(calldata),
        _ => Err(ServerlessError::InvalidTxCalldataType),
    }?;

    // hex decode calldata by skipping to the code bytes
    let mut calldata = hex::decode(&calldata[138..])?;

    // strip trailing zeros
    let idx = calldata.iter().rev().position(|x| *x != 0).unwrap_or(0);
    calldata.truncate(calldata.len() - idx);

    // write calldata to file
    let mut file =
        File::create(workerd_runtime_path.to_owned() + "/" + tx_hash + "-" + slug + ".js")
            .await
            .map_err(ServerlessError::CodeFileCreate)?;
    file.write_all(calldata.as_slice())
        .await
        .map_err(ServerlessError::CodeFileCreate)?;
    Ok(())
}

pub async fn create_config_file(
    tx_hash: &str,
    slug: &str,
    workerd_runtime_path: &str,
    free_port: u16,
) -> Result<(), ServerlessError> {
    let capnp_data = format!(
        "
using Workerd = import \"/workerd/workerd.capnp\";

const oysterConfig :Workerd.Config = (
  services = [ (name = \"main\", worker = .oysterWorker) ],
  sockets = [ ( name = \"http\", address = \"*:{free_port}\", http = (), service = \"main\" ) ]
);

const oysterWorker :Workerd.Worker = (
  serviceWorkerScript = embed \"{tx_hash}-{slug}.js\",
  compatibilityDate = \"2022-09-16\",
);"
    );

    let mut file =
        File::create(workerd_runtime_path.to_owned() + "/" + tx_hash + "-" + slug + ".capnp")
            .await
            .map_err(ServerlessError::ConfigFileCreate)?;
    file.write_all(capnp_data.as_bytes())
        .await
        .map_err(ServerlessError::ConfigFileCreate)?;
    Ok(())
}

pub fn get_port(cgroup: &str) -> Result<u16, ServerlessError> {
    u16::from_str_radix(&cgroup[8..], 10)
        .map(|x| x + 11000)
        .map_err(ServerlessError::BadPort)
}

// TODO: timeouts?
pub async fn execute(
    tx_hash: &str,
    slug: &str,
    workerd_runtime_path: &str,
    cgroup: &str,
) -> Result<Child, ServerlessError> {
    let args = [
        &(workerd_runtime_path.to_owned() + "/workerd"),
        "serve",
        &(workerd_runtime_path.to_owned() + "/" + tx_hash + "-" + slug + ".capnp"),
        "--verbose",
    ];

    Ok(Cgroups::execute(cgroup, args)?)
}

pub async fn wait_for_port(port: u16) -> bool {
    let start_time = Instant::now();

    while start_time.elapsed() < Duration::from_secs(1) {
        match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
            Ok(_) => return true,
            Err(_) => sleep(Duration::from_millis(1)).await,
        }
    }
    false
}

pub async fn cleanup_code_file(
    tx_hash: &str,
    slug: &str,
    workerd_runtime_path: &str,
) -> Result<(), ServerlessError> {
    tokio::fs::remove_file(workerd_runtime_path.to_owned() + "/" + tx_hash + "-" + slug + ".js")
        .await
        .map_err(ServerlessError::CodeFileDelete)?;
    Ok(())
}

pub async fn cleanup_config_file(
    tx_hash: &str,
    slug: &str,
    workerd_runtime_path: &str,
) -> Result<(), ServerlessError> {
    tokio::fs::remove_file(workerd_runtime_path.to_owned() + "/" + tx_hash + "-" + slug + ".capnp")
        .await
        .map_err(ServerlessError::ConfigFileDelete)?;
    Ok(())
}

pub async fn get_workerd_response(
    port: u16,
    req: HttpRequest,
    body: actix_web::web::Bytes,
) -> Result<HttpResponse, anyhow::Error> {
    let port_str = port.to_string();
    let req_url = "http://127.0.0.1:".to_string() + &port_str + "/";
    let client = reqwest::Client::new();
    let response = req
        .headers()
        .into_iter()
        .fold(
            client.request(req.method().clone(), req_url),
            |req, header| req.header(header.0.clone(), header.1.clone()),
        )
        .body(body)
        .send()
        .await?;

    let actix_resp = response
        .headers()
        .into_iter()
        .fold(
            HttpResponse::build(response.status()),
            |mut resp, header| {
                resp.append_header((header.0.clone(), header.1.clone()));
                resp
            },
        )
        .body(response.bytes().await?);
    Ok(actix_resp)
}
