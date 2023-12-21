use actix_web::{web, App, HttpServer};
use anyhow::{anyhow, Context};
use clap::Parser;
use tokio::fs;

use serverless::cgroups::Cgroups;
use serverless::model::AppState;

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
        default_value = "https://goerli-rollup.arbitrum.io/rpc"
    )]
    rpc: String,

    #[clap(
        long,
        value_parser,
        default_value = "0x30694a76d737211a908d0dd672f47e1d29fbfb02"
    )]
    contract: String,

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

    let signer = k256::SecretKey::from_slice(
        fs::read(cli.signer)
            .await
            .context("failed to read signer key")?
            .as_slice(),
    )
    .context("invalid signer key")?;

    let app_data = web::Data::new(AppState {
        cgroups: cgroups.into(),
        running: std::sync::atomic::AtomicBool::new(true),
        runtime_path: cli.runtime_path,
        rpc: cli.rpc,
        contract: cli.contract,
        signer,
    });

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

    Ok(())
}
