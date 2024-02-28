mod web;
mod cache;

use actix_files as fs;
use actix_web::{App, HttpServer};
use clap::Parser;

#[derive(Parser)]
struct Args {

    // Port of the Application
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
    // The Host Interface
    #[arg(short, long, default_value_t = ("127.0.0.1".to_string()))]
    host: String
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    let current_dir = std::env::current_exe()
        .unwrap()
        .as_path()
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    Ok(HttpServer::new(move || {
        App::new().service(web::calendar::get_events).service(
            fs::Files::new("/", &(current_dir.clone() + "/static/")).index_file("index.html"),
        )
    })
    .bind((args.host, args.port))?
    .run()
    .await?)
}
