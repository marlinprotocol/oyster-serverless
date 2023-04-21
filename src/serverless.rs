use reqwest::Response;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::{Error};
use serde_json::json;
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
    let child = Command::new("/usr/bin/cgexec")
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

//Fetch the js code from the storage server
pub async fn get_attestation_doc() -> Result<Response, Box<dyn std::error::Error>> {
    let req_url = "http://127.0.0.1:1300".to_string();
    let client = reqwest::Client::new();
    let response = client.get(req_url).send().await?;
    Ok(response)
}

//Fetching js code from the storage server
pub async fn get_code_from_storage_server(attestation_doc:&str,id:&str) -> Result<Response, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = "http://example.com";
    let body = json!({
        "key": id
    });

    let mut headers = HeaderMap::new();
    headers.insert("Attestation", HeaderValue::from_str(attestation_doc)?);

    let response = client.post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    Ok(response)
}