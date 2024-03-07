use std::time::{Duration, SystemTime};

use filetime;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;

use crate::error::ServerlessError;

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

async fn get_number_of_code_files(workerd_runtime_path: &str) -> Result<i32, tokio::io::Error> {
    let mut reader = fs::read_dir(workerd_runtime_path.to_owned()).await?;

    let mut count = 0;
    while let Some(entry) = reader.next_entry().await? {
        let path = entry.path();
        if path.is_file()
            && path.file_name().map_or(false, |name| {
                let name = name.to_string_lossy();
                name.starts_with("0x") && name.ends_with(".js")
            })
        {
            count += 1;
        }
    }

    Ok(count)
}

async fn delete_oldest_code_file(workerd_runtime_path: &str) -> Result<(), tokio::io::Error> {
    let mut file_to_delete = String::new();
    let mut file_modified_time = SystemTime::now();

    let mut reader = fs::read_dir(workerd_runtime_path.to_owned()).await?;

    while let Some(entry) = reader.next_entry().await? {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();

        if file_name.starts_with("0x") && file_name.ends_with(".js") {
            let metadata = fs::metadata(path.clone()).await.unwrap();
            let modified_time = metadata.modified().unwrap();

            if modified_time < file_modified_time {
                file_to_delete = file_name.to_string();
                file_modified_time = modified_time;
            }
        }
    }

    if let Err(err) = fs::remove_file(workerd_runtime_path.to_owned() + "/" + &file_to_delete).await
    {
        if err.kind() != std::io::ErrorKind::NotFound {
            return Err(err);
        }
    }
    Ok(())
}

pub async fn create_code_file(
    tx_hash: &str,
    workerd_runtime_path: &str,
    rpc: &str,
    contract: &str,
) -> Result<(), ServerlessError> {
    if fs::metadata(&(workerd_runtime_path.to_owned() + "/" + tx_hash + ".js"))
        .await
        .is_ok()
    {
        let current_time = filetime::FileTime::now();
        filetime::set_file_times(
            &(workerd_runtime_path.to_owned() + "/" + tx_hash + ".js"),
            current_time,
            current_time,
        )
        .map_err(ServerlessError::UpdateModifiedTime)?;
        return Ok(());
    }

    let no_of_files = get_number_of_code_files(workerd_runtime_path)
        .await
        .map_err(ServerlessError::CodeFileCreate)?;

    if no_of_files >= 40 {
        delete_oldest_code_file(workerd_runtime_path)
            .await
            .map_err(ServerlessError::CodeFileDelete)?;
    }

    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(workerd_runtime_path.to_owned() + "/temp-" + tx_hash + ".js")
        .await
    {
        Ok(mut file) => {
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
            if contract_address != contract {
                return Err(ServerlessError::InvalidTxToValue(
                    contract_address,
                    contract.to_owned(),
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
            file.write_all(calldata.as_slice())
                .await
                .map_err(ServerlessError::CodeFileCreate)?;

            // rename file
            fs::rename(
                workerd_runtime_path.to_owned() + "/temp-" + tx_hash + ".js",
                workerd_runtime_path.to_owned() + "/" + tx_hash + ".js",
            )
            .await
            .map_err(ServerlessError::ConfigFileCreate)?;

            return Ok(());
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                let mut file_exists = false;
                let mut count = 0;
                while !file_exists && count < 30 {
                    sleep(Duration::from_millis(50)).await;
                    file_exists = tokio::fs::metadata(
                        workerd_runtime_path.to_owned() + "/" + tx_hash + ".js",
                    )
                    .await
                    .is_ok();
                    count += 1;
                }
                if file_exists {
                    return Ok(());
                }
            }
            return Err(ServerlessError::CodeFileCreate(e));
        }
    }
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
  modules = [
    (name = \"main\", esModule = embed \"{tx_hash}.js\")
  ],
  compatibilityDate = \"2023-03-07\",
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

pub async fn cleanup_config_file(
    tx_hash: &str,
    slug: &str,
    workerd_runtime_path: &str,
) -> Result<(), ServerlessError> {
    fs::remove_file(workerd_runtime_path.to_owned() + "/" + tx_hash + "-" + slug + ".capnp")
        .await
        .map_err(ServerlessError::ConfigFileDelete)?;
    Ok(())
}
