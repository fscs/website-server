mod cache;

use std::{convert::identity, str::FromStr};

mod database;
mod domain;
mod web;

use crate::database::DatabasePool;
use anyhow::anyhow;
use clap::Parser;
use lazy_static::lazy_static;
use log::LevelFilter;

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

    let dir = get_base_dir()?;

    let database_url = ARGS.database_url.clone().map_or(
        std::env::var("DATABASE_URL").map_or("postgres://postgres:postgres@localhost/postgres".to_string(), identity),
        identity
    );

    let database = DatabasePool::new(&database_url).await?;
    sqlx::migrate!().run(database.pool()).await?;

    web::start_server(dir, database).await
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
