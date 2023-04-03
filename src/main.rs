mod handler;
mod model;
mod response;
mod serverless;

use actix_web::{App, HttpServer};
use dotenv::dotenv;

use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let port: u16 = env::var("PORT")
        .unwrap()
        .parse::<u16>()
        .expect("PORT must be a valid number");

    log::info!("Make sure you have done the cgroup setup on your system");

    let server = HttpServer::new(move || App::new().configure(handler::config))
        .bind(("0.0.0.0", port))
        .unwrap_or_else(|_| panic!("Can not bind to {}", &port))
        .run();

    log::info!("Server started on port {}", port);

    server.await
}
