use crate::response::WorkerdDataResponse;
use ethers::abi::decode;
use ethers::abi::ParamType;
use ethers::core::utils::hex::decode as hex_decode;
use reqwest::{Client, Error};
use serde_json::{json, Value};
use std::fs::remove_file;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use vsock::{VsockAddr, VsockStream, VMADDR_CID_ANY};

//Get a free port for running workerd
pub fn get_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

//Decoding calldata using ethers
pub fn decode_call_data(json_data: &str) -> Result<String, Box<dyn std::error::Error>> {
    let json_response_size = json_data.len();
    let call_data = &json_data[11..json_response_size - 1];
    let vec1 = hex_decode(call_data).unwrap();
    let data: &[u8] = vec1.as_slice();
    let result = decode(vec![ParamType::String].as_slice(), data).unwrap();
    let decoded_calldata = result[0].to_string();
    Ok(decoded_calldata)
}

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

    let json_response = response.json::<Value>().await;

    match json_response {
        Ok(data) => Ok(data),
        Err(e) => Err(e),
    }
}

//Wait for a port to bind
pub fn wait_for_port(port: u16) -> bool {
    let start_time = Instant::now();

    while start_time.elapsed() < Duration::from_secs(5) {
        match TcpStream::connect(format!("127.0.0.1:{}", port)) {
            Ok(_) => return true,
            Err(_) => sleep(Duration::from_millis(1)),
        }
    }
    false
}

//Fetching response from the workerd runtime using a http request
pub async fn get_workerd_response(port: u16) -> Result<String, Box<dyn std::error::Error>> {
    let port_str = port.to_string();
    let req_url = "http://127.0.0.1:".to_string() + &port_str + "/";
    let response = reqwest::get(req_url.to_string()).await?;
    let body = response.text().await?;
    Ok(body)
}

//Generating a capnp configuration file
pub async fn create_capnp_file(
    tx_hash: &str,
    free_port: u16,
    workerd_runtime_path: &str,
) -> std::io::Result<()> {
    let capnp_data = format!(
        "using Workerd = import \"/workerd/workerd.capnp\";

    const helloWorldExample :Workerd.Config = (
      services = [ (name = \"main\", worker = .helloWorld) ],
      sockets = [ ( name = \"http\", address = \"*:{free_port}\", http = (), service = \"main\" ) ]
    );
    
    const helloWorld :Workerd.Worker = (
      serviceWorkerScript = embed \"{tx_hash}.js\",
      compatibilityDate = \"2022-09-16\",
    );"
    );

    let mut file = File::create(workerd_runtime_path.to_string() + tx_hash + ".capnp").await?;
    file.write_all(capnp_data.as_bytes()).await?;
    Ok(())
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

//Running users js code using workerd and the generated config file
pub async fn run_workerd_runtime(
    tx_hash: &str,
    workerd_runtime_path: &str,
) -> Result<Child, Box<dyn std::error::Error>> {
    let child = Command::new(workerd_runtime_path.to_string() + "workerd")
        .arg("serve")
        .arg(workerd_runtime_path.to_string() + tx_hash + ".capnp")
        .spawn()?;
    Ok(child)
}

//Deleting files (js,capnp) once the response is generated
pub fn delete_file(file_path: &str) -> Result<(), Error> {
    let _remove_file = remove_file(file_path);
    Ok(())
}

//Creating a vsock socket and sending workerd data over vsock
pub fn send_json_over_vsock(data: &WorkerdDataResponse) -> Result<(), Box<dyn std::error::Error>> {

    let mut socket = VsockStream::connect(&VsockAddr::new(VMADDR_CID_ANY, 5500))?;

    let json_data = serde_json::to_string(data)?;
    socket.write_all(json_data.as_bytes())?;

    Ok(())
}
