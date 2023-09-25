mod handler;
mod model;
mod response;
mod serverless;
mod tests;

use crate::model::AppState;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;

use std::env;
use std::sync::Mutex;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let gateway_url = env::var("GATEWAY").unwrap();
    let client = awc::Client::default();
    let response = client
        .get("http://".to_owned() + &gateway_url + "/register")
        .insert_header(("worker-id", "test"))
        .send()
        .await;

    println!("{:?}", response);

    let port: u16 = env::var("PORT")
        .unwrap()
        .parse::<u16>()
        .expect("PORT must be a valid number");

    let cgroup_version: u8 = env::var("CGROUP_VERSION")
        .unwrap()
        .parse::<u8>()
        .expect("CGROUP VERSION must be a valid number ( Options: 1 or 2)");

    let cgroup_list = serverless::get_cgroup_list(cgroup_version).unwrap();
    if cgroup_list.is_empty() {
        log::error!("No cgroups found. Make sure you have generated cgroups on your system by following the instructions in the readme file.");
        std::process::exit(1);
    }

    let app_data = web::Data::new(AppState {
        cgroup_list: cgroup_list.clone(),
        cgroup_version,
        running: Mutex::new(true),
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
