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
    let port: u16 = env::var("PORT")
        .unwrap()
        .parse::<u16>()
        .expect("PORT must be a valid number");
    let server = HttpServer::new(|| App::new().configure(handler::config))
        .bind(("0.0.0.0", port))?
        .run();
    println!("Server started on port {}", port);
    server.await
}
