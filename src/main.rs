use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serverless::cgroups::Cgroups;
use serverless::model::AppState;
use serverless::BillingContract;

use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::{anyhow, Context};
use clap::Parser;
use ethers::providers::{Http, Provider};
use ethers::types::Address;
use tokio::fs;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(long, value_parser, default_value = "6001")]
    port: u16,

    #[clap(long, value_parser)]
    billing_port: u16, // TODO: ADD THE DEFAULT PORT

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
    billing_contract: String, // TODO: ADD A DEFAULT ADDRESS

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
    let billing_port: u16 = cli.billing_port;

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

    let rpc_provider = Provider::<Http>::try_from(&cli.rpc)
        .context("Failed to connect to the rpc")?
        .interval(Duration::from_millis(1000));
    let billing_contract = BillingContract::new(
        cli.billing_contract
            .parse::<Address>()
            .context("Failed to parse billing contract address")?,
        Arc::new(rpc_provider),
    );

    let app_data = web::Data::new(AppState {
        cgroups: cgroups.into(),
        running: std::sync::atomic::AtomicBool::new(true),
        runtime_path: cli.runtime_path,
        rpc: cli.rpc,
        contract: cli.contract,
        signer: signer,
        billing_contract: billing_contract,
        execution_costs: HashMap::new().into(),
        last_bill_claim: (None, None).into(),
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

    let billing_server = HttpServer::new(move || {
        App::new()
            .app_data(app_data_clone.clone())
            .service(serverless::billing_handler::inspect_bill)
            .service(serverless::billing_handler::get_last_bill_claim)
            .service(serverless::billing_handler::export_bill)
            .default_service(web::to(HttpResponse::NotFound))
    })
    .bind(("0.0.0.0", billing_port))
    .context(format!("could not bind to port {billing_port}"))?
    .run();

    println!("Billing Server started on port {}", billing_port);

    tokio::try_join!(server, billing_server)?;

    Ok(())
}
