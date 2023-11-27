use reqwest::Client;
use serde_json::{json, Value};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::ServerlessError;

async fn get_transaction_data(tx_hash: &str) -> Result<Value, reqwest::Error> {
    let client = Client::new();
    let url = "https://goerli-rollup.arbitrum.io/rpc";
    let method = "eth_getTransactionByHash";
    let params = json!([&tx_hash]);
    let id = 1;

    let request = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id,
    });

    let response = client.post(url).json(&request).send().await?;
    let json_response = response.json::<Value>().await?;

    Ok(json_response)
}

async fn create_code_file(
    tx_hash: &str,
    slug: &str,
    workerd_runtime_path: &str,
) -> Result<(), ServerlessError> {
    // get tx data
    let mut tx_data = match get_transaction_data(tx_hash).await?["result"].take() {
        Value::Null => Err(ServerlessError::TxNotFound),
        other => Ok(other),
    }?;

    // get contract address
    let contract_address = match tx_data["to"].take() {
        Value::String(value) => Ok(value),
        _ => Err(ServerlessError::InvalidTxToType),
    }?;

    // check contract address matches expected
    if contract_address != "\"0x30694a76d737211a908d0dd672f47e1d29fbfb02\"" {
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

    // decode calldata
    let calldata = hex::decode(calldata)?;

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

async fn create_config_file(
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
  serviceWorkerScript = embed \"{tx_hash}.js\",
  compatibilityDate = \"2022-09-16\",
);"
    );

    let mut file =
        File::create(workerd_runtime_path.to_string() + "/" + tx_hash + "-" + slug + ".capnp")
            .await
            .map_err(ServerlessError::ConfigFileCreate)?;
    file.write_all(capnp_data.as_bytes())
        .await
        .map_err(ServerlessError::ConfigFileCreate)?;
    Ok(())
}
