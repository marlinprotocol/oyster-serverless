use std::collections::HashMap;
use actix_web::{web, App, HttpServer};
use anyhow::{anyhow, Context};
use clap::Parser;
use ethers::abi::Abi;
use serverless::billing_job;
use tiny_keccak::Keccak;
use tokio::fs;

use serverless::cgroups::Cgroups;
use serverless::model::AppState;
use tokio::time::{interval, Duration};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(long, value_parser, default_value = "6001")]
    port: u16,

    #[clap(long, value_parser, default_value = "./runtime/")]
    runtime_path: String,

    #[clap(long, value_parser, default_value = "www.marlin.org")]
    gateway: String,

    #[clap(
        long,
        value_parser,
        default_value = "https://sepolia-rollup.arbitrum.io/rpc"
    )]
    rpc: String,

    #[clap(
        long,
        value_parser,
        default_value = "0x44fe06d2940b8782a0a9a9ffd09c65852c0156b1"
    )]
    contract: String,

    #[clap(long, value_parser)]
    operator_key: String,

    #[clap(long, value_parser)]
    signer: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Args::parse();

    // let gateway_url = cli.gateway;
    // let client = awc::Client::default();
    // let response = client
    //     .get("http://".to_owned() + &gateway_url + "/register")
    //     .insert_header(("worker-id", "test"))
    //     .send()
    //     .await;

    // println!("{:?}", response);

    let port: u16 = cli.port;

    let cgroups = Cgroups::new().context("failed to construct cgroups")?;
    if cgroups.free.is_empty() {
        return Err(anyhow!("no cgroups found, make sure you have generated cgroups on your system using instructions in the readme"));
    }

    let signer = k256::ecdsa::SigningKey::from_slice(
        fs::read(cli.signer)
            .await
            .context("failed to read signer key")?
            .as_slice(),
    )
    .context("invalid signer key")?;
             
    let abi_json = fs::read_to_string("src/contract_abi.json")
                            .await
                            .context("failed to read contract ABI")?;
    let abi = serde_json::from_str::<Abi>(&abi_json).context("failed to deserialize ABI")?;

    let app_data = web::Data::new(AppState {
        cgroups: cgroups.into(),
        running: std::sync::atomic::AtomicBool::new(true),
        runtime_path: cli.runtime_path,
        rpc: cli.rpc,
        contract: cli.contract,
        signer: signer,
        abi: abi,
        operator_key: cli.operator_key,
        service_costs: HashMap::new().into(),
        hasher: Keccak::v256().into(),
    });
    let app_data_clone = app_data.clone();

    let server = HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .default_service(web::to(serverless::handler::serverless))
    })
    .bind(("0.0.0.0", port))
    .context(format!("could not bind to port {port}"))?
    .run();

    println!("Server started on port {}", port);

    server.await?;

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(600));           // TODO: FIX THE REGULAR INTERVAL 
        loop {
            interval.tick().await;

            if !app_data_clone.service_costs.lock().await.is_empty() {
                match billing_job::billing_scheduler(app_data_clone.clone()).await {
                    Ok(tx_hash) => println!("Transaction sent for billing: {}", tx_hash),
                    Err(err) => println!("Error while sending billing transaction: {:?}", err),
                }
            }
        }
    });

    Ok(())
}
