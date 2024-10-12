#![warn(clippy::shadow_unrelated)]

use clap::Parser;
use log::LevelFilter;
use std::{convert::identity, path::PathBuf, str::FromStr, sync::LazyLock};

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
    #[arg(long, default_value_t = {"Info".to_string()})]
    log_level: String,
    #[arg(short, long)]
    database_url: Option<String>,
    #[arg(short, long)]
    auth_url: String,
    #[arg(short, long)]
    token_url: String,
    #[arg(short, long)]
    user_info: String,
    #[arg(short = 'j', long)]
    workers: Option<usize>,
}

struct ContentDir {
    public: PathBuf,
    hidden: PathBuf,
    protected: PathBuf,
}

static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);
static CONTENT_DIR: LazyLock<ContentDir> = LazyLock::new(|| ContentDir {
    public: ARGS.content_dir.join("public"),
    hidden: ARGS.content_dir.join("hidden"),
    protected: ARGS.content_dir.join("protected"),
});

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

    let mut transaction = database.start_transaction().await?;
    sqlx::migrate!().run(&mut *transaction).await?;
    transaction.commit().await?;

    let _ = web::start_server(database).await;
    Ok(())
}
