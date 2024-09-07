#![warn(clippy::shadow_unrelated)]

use std::{convert::identity, path::PathBuf, str::FromStr};
use clap::Parser;
use lazy_static::lazy_static;
use log::LevelFilter;

mod cache;
mod database;
mod domain;
mod web;

use crate::database::DatabasePool;

#[derive(Parser)]
struct Args {
    // Port of the Application
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
    //The Host Interface
    #[arg(long, default_value_t = {"127.0.0.1".to_string()})]
    host: String,
    #[arg(long, required = true)]
    content_dir: PathBuf,
    #[arg(long, required = true)]
    private_content_dir: PathBuf,
    #[arg(long, required = true)]
    hidden_content_dir: PathBuf,
    #[arg(long, default_value_t = {"Info".to_string()})]
    log_level: String,
    #[arg(short, long)]
    database_url: Option<String>,
}

lazy_static! {
    static ref ARGS: Args = Args::parse();
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::from_str(&ARGS.log_level)?)
        .init();

    let database_url = ARGS.database_url.clone().map_or(
        std::env::var("DATABASE_URL").map_or(
            "postgres://postgres:postgres@localhost/postgres".to_string(),
            identity,
        ),
        identity,
    );

    let database = DatabasePool::new(&database_url).await?;
    sqlx::migrate!().run(database.pool()).await?;

    let _ = web::start_server(database).await;
    Ok(())
}
