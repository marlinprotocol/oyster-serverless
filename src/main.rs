mod handler;
mod model;
mod response;
mod serverless;
mod tests;

use crate::model::AppState;
use actix_web::{web, App, HttpServer};

use clap::Parser;

/// Oyster Serverless
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // port for web server
    #[clap(long, value_parser, default_value = "6001")]
    port: u16,

    // path to workerd runtime
    #[clap(long, value_parser, default_value = "./runtime/")]
    runtime_path: String,

    // cgroup version
    #[clap(long, value_parser, default_value = "2")]
    cgroup_version: u8,

    // gateway address url
    #[clap(long, value_parser, default_value = "www.marlin.org")]
    gateway: String,
}
use std::sync::Mutex;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = Args::parse();
    
    let gateway_url = cli.gateway;
    let client = awc::Client::default();
    let response = client
        .get("http://".to_owned() + &gateway_url + "/register")
        .insert_header(("worker-id", "test"))
        .send()
        .await;

    println!("{:?}", response);

    let port: u16 = cli.port;

    let cgroup_version: u8 = cli.cgroup_version;

    let cgroup_list = serverless::get_cgroup_list(cgroup_version).unwrap();
    if cgroup_list.is_empty() {
        log::error!("No cgroups found. Make sure you have generated cgroups on your system by following the instructions in the readme file.");
        std::process::exit(1);
    }

    let app_data = web::Data::new(AppState {
        cgroup_list: cgroup_list.clone(),
        cgroup_version,
        running: Mutex::new(true),
        runtime_path: cli.runtime_path,
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .configure(handler::config)
    })
    .bind(("0.0.0.0", port))
    .unwrap_or_else(|_| panic!("Can not bind to {}", &port))
    .run();

    log::info!("Server started on port {}", port);

    server.await
}
