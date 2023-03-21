mod handler;
mod model;
mod response;
mod serverless;

use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use slog::{info, o, Drain, Logger};
use std::env;

fn configure_log() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();

    let console_drain = slog_term::FullFormat::new(decorator).build().fuse();

    let console_drain = slog_async::Async::new(console_drain).build().fuse();
    slog::Logger::root(console_drain, o!())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let log = configure_log();
    let port: u16 = env::var("PORT")
        .unwrap()
        .parse::<u16>()
        .expect("PORT must be a valid number");

    info!(log, "Server started on port {}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(log.clone()))
            .configure(handler::config)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
