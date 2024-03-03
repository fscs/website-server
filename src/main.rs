mod cache;
mod web;

use actix_files as fs;
use actix_web::{App, HttpServer};
use clap::Parser;
use lazy_static::lazy_static;

#[derive(Parser)]
struct Args {
    // Port of the Application
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
    // The Host Interface
    #[arg(short, long, default_value_t = {"127.0.0.1".to_string()})]
    host: String,
    //Use the Directory of the executable as Base Directory instead of the working Directory
    #[arg(long, default_value_t = false)]
    use_executable_dir: bool,
}

lazy_static! {
    static ref ARGS: Args = Args::parse();
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let dir = if !ARGS.use_executable_dir {
        std::env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    } else {
        std::env::current_exe()
            .unwrap()
            .as_path()
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    };

    let server = HttpServer::new(move || {
        App::new()
            .service(web::calendar::service("/api/calendar"))
            .service(fs::Files::new("/", &(dir.clone() + "/static/")).index_file("index.html"))
    }).bind((ARGS.host.as_str(), ARGS.port))?;

    println!("running server on port {} bound to {}", ARGS.port, ARGS.host);
    
    Ok(server.run().await?)
}
