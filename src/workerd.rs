use std::process::Child;
use std::time::{Duration, Instant};

use actix_web::{HttpRequest, HttpResponse};
use k256::elliptic_curve::generic_array::sequence::Lengthen;
use reqwest::redirect::Policy;
use tiny_keccak::{Hasher, Keccak};
use tokio::net::TcpStream;
use tokio::time::sleep;

use crate::cgroups::Cgroups;
use crate::error::ServerlessError;

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

pub async fn get_workerd_response(
    port: u16,
    req: HttpRequest,
    body: actix_web::web::Bytes,
    signer: &k256::ecdsa::SigningKey,
    host_header: &str,
) -> Result<HttpResponse, anyhow::Error> {
    let mut hasher = Keccak::v256();
    hasher.update(b"|oyster-serverless-hasher|");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    hasher.update(b"|timestamp|");
    hasher.update(&timestamp.to_be_bytes());

    hasher.update(b"|request|");
    hasher.update(b"|method|");
    hasher.update(req.method().to_string().as_bytes());
    hasher.update(b"|pathandquery|");
    hasher.update(
        req.uri()
            .path_and_query()
            .map(|x| x.as_str())
            .unwrap_or("")
            .as_bytes(),
    );
    hasher.update(b"|host|");
    hasher.update(host_header.as_bytes());
    hasher.update(b"|body|");
    hasher.update(&body);

    let port_str = port.to_string();
    let req_url = "http://127.0.0.1:".to_string() + &port_str + "/";
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()?;
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
    hasher.update(b"|response|");

    let mut actix_resp = response.headers().into_iter().fold(
        HttpResponse::build(response.status()),
        |mut resp, header| {
            resp.append_header((header.0.clone(), header.1.clone()));
            resp
        },
    );
    let response_body = response.bytes().await?;

    hasher.update(b"|body|");
    hasher.update(&response_body);

    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    let (rs, v) = signer.sign_prehash_recoverable(&hash)?;

    let signature = rs.to_bytes().append(27 + v.to_byte());

    actix_resp.insert_header(("X-Oyster-Timestamp", timestamp.to_string()));
    actix_resp.insert_header(("X-Oyster-Signature", hex::encode(signature.as_slice())));

    Ok(actix_resp.body(response_body))
}
