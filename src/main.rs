
mod cache;
use std::str::FromStr;
mod database;
mod web;
mod domain;

use crate::database::DatabasePool;
use actix_files as fs;
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use anyhow::anyhow;
use clap::Parser;
use lazy_static::lazy_static;
use log::{LevelFilter};
use web::topmanager;

#[derive(Parser)]
struct Args {
    // Port of the Application
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
    //The Host Interface
    #[arg(long, default_value_t = {"127.0.0.1".to_string()})]
    host: String,
    //Use the Directory of the executable as Base Directory instead of the working Directory
    #[arg(long, default_value_t = false)]
    use_executable_dir: bool,
    #[arg(long, default_value_t = {"Info".to_string()})]
    log_level: String,
    #[arg(short, long, default_value_t = {"postgres://postgres:postgres@localhost/postgres".to_string()})]
    database_url: String,
}

lazy_static! {
    static ref ARGS: Args = Args::parse();
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::from_str(&ARGS.log_level)?)
        .init();

    let dir = get_base_dir()?;

    let database = DatabasePool::new(&ARGS.database_url).await?;
    sqlx::migrate!().run(database.pool()).await?;

    Ok(HttpServer::new(move || {
        App::new()
            .service(web::calendar::service("/api/calendar"))
            .service(topmanager::service("/topmanager"))
            .service(fs::Files::new("/", &(dir.clone() + "/static/")).index_file("index.html"))
            .app_data(Data::new(database.clone()))
    })
    .bind((ARGS.host.as_str(), ARGS.port))?
    .run()
    .await?)
}

fn get_base_dir() -> anyhow::Result<String> {
    Ok(if !ARGS.use_executable_dir {
        std::env::current_dir()?
            .to_str()
            .ok_or(anyhow!("Working Directory Contains non UTF-8 Characters"))?
            .to_string()
    } else {
        std::env::current_exe()?
            .as_path()
            .parent()
            .ok_or(anyhow!("Executable has no Parent Directory"))?
            .to_str()
            .ok_or(anyhow!(
                "Directory of the Executable Contains non UTF-8 Characters"
            ))?
            .to_string()
    })
}
