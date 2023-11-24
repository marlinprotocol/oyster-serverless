use ethers::abi::decode;
use ethers::abi::ParamType;
use ethers::core::utils::hex::decode as hex_decode;
use reqwest::Response;
use reqwest::{Client, Error};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::fs::remove_file;
use std::net::TcpListener;
use std::net::TcpStream;
use std::process::Stdio;
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

//Fetching the calldata using the txhash provided by the user
pub async fn get_transaction_data(_tx_hash: &str) -> Result<Value, Error> {
    let client = Client::new();
    let url = "https://goerli-rollup.arbitrum.io/rpc";
    let method = "eth_getTransactionByHash";
    let params = json!([&_tx_hash]);
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

//Decoding calldata using ethers
pub fn decode_call_data(json_data: &str) -> Result<String, Box<dyn std::error::Error>> {
    //Remove the first 11 characters and last 1 character from the string
    let json_response_size = json_data.len();
    let call_data = &json_data[11..json_response_size - 1];
    //Convert hex string to byte array
    let vec1 = hex_decode(call_data).unwrap();
    let data: &[u8] = vec1.as_slice();
    //Decode the byte array using ethers
    let result = decode(vec![ParamType::String].as_slice(), data).unwrap();
    //Convert the decoded calldata to string
    let decoded_calldata = result[0].to_string();
    Ok(decoded_calldata)
}

//Get a free port for running workerd
pub fn get_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

//Generating the js file using the decoded calldata
pub async fn create_js_file(
    decoded_calldata: &str,
    tx_hash: &str,
    workerd_runtime_path: &str,
) -> std::io::Result<()> {
    let mut file = File::create(workerd_runtime_path.to_string() + tx_hash + ".js").await?;
    file.write_all(decoded_calldata.as_bytes()).await?;
    Ok(())
}

//Generating a capnp configuration file
pub async fn create_capnp_file(
    tx_hash: &str,
    free_port: u16,
    workerd_runtime_path: &str,
) -> std::io::Result<()> {
    let capnp_data = format!(
        "using Workerd = import \"/workerd/workerd.capnp\";

    const oysterServerlessConfig :Workerd.Config = (
      services = [ (name = \"main\", worker = .oysterServerless) ],
      sockets = [ ( name = \"http\", address = \"*:{free_port}\", http = (), service = \"main\" ) ]
    );
    
    const oysterServerless :Workerd.Worker = (
      serviceWorkerScript = embed \"{tx_hash}.js\",
      compatibilityDate = \"2022-09-16\",
    );"
    );

    let mut file = File::create(workerd_runtime_path.to_string() + tx_hash + ".capnp").await?;
    file.write_all(capnp_data.as_bytes()).await?;
    Ok(())
}

//Fetching an available cgroup from the list of cgroups generated at boot
pub fn find_available_cgroup(
    cgroup_version: u8,
    cgroup_list: &[String],
) -> Result<String, std::io::Error> {
    for cgroup_name in cgroup_list.iter() {
        let mut cgroup_path = "/sys/fs/cgroup/memory/".to_string() + cgroup_name + "/cgroup.procs";
        if cgroup_version == 2 {
            cgroup_path = "/sys/fs/cgroup/".to_string() + cgroup_name + "/cgroup.procs";
        }

        let running_processes = fs::read_to_string(cgroup_path)?;

        if running_processes.is_empty() {
            return Ok(cgroup_name.to_string());
        }
    }

    Ok(String::from("No available cgroup"))
}

//Running users code using workerd inside a generated cgroup
pub async fn run_workerd_runtime(
    file_name: &str,
    workerd_runtime_path: &str,
    available_cgroup: &str,
) -> Result<Child, Box<dyn std::error::Error>> {
    let child = Command::new("sudo")
        .arg("/usr/bin/cgexec")
        .arg("-g")
        .arg("memory:".to_string() + available_cgroup)
        .arg(workerd_runtime_path.to_string() + "workerd")
        .arg("serve")
        .arg(workerd_runtime_path.to_string() + file_name + ".capnp")
        .arg("--verbose")
        .stderr(Stdio::piped())
        .spawn()?;
    Ok(child)
}

//Wait for a port to bind
pub fn wait_for_port(port: u16) -> bool {
    let start_time = Instant::now();

    while start_time.elapsed() < Duration::from_secs(1) {
        match TcpStream::connect(format!("127.0.0.1:{}", port)) {
            Ok(_) => return true,
            Err(_) => sleep(Duration::from_millis(1)),
        }
    }
    false
}

//Fetching response from the workerd runtime using a http request
pub async fn get_workerd_response(
    port: u16,
    input: Option<HashMap<String, serde_json::Value>>,
) -> Result<Response, Box<dyn std::error::Error>> {
    let port_str = port.to_string();
    let req_url = "http://127.0.0.1:".to_string() + &port_str + "/";
    let client = reqwest::Client::new();
    let response = client
        .post(req_url)
        .header("Content-Type", "application/json")
        .json(&input)
        .send()
        .await?;
    Ok(response)
}

//Deleting files (js,capnp) once the response is generated
pub fn delete_file(file_path: &str) -> Result<(), Error> {
    let _remove_file = remove_file(file_path);
    Ok(())
}

//Get cgroup list
pub fn get_cgroup_list(cgroup_version: u8) -> Result<Vec<String>, std::io::Error> {
    let mut cgroup_list: Vec<String> = Vec::new();
    let mut cgroup_path = "/sys/fs/cgroup/memory";
    if cgroup_version == 2 {
        cgroup_path = "/sys/fs/cgroup";
    }

    let dir_entries = fs::read_dir(cgroup_path).unwrap();
    for entry in dir_entries {
        let path = entry.unwrap().path();
        if let Some(filename) = path.file_name() {
            if let Some(name) = filename.to_str() {
                if name.starts_with("workerd") {
                    cgroup_list.push(name.to_string());
                }
            }
        }
    }

    Ok(cgroup_list)
}
