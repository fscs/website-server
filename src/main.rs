#![warn(clippy::shadow_unrelated)]

use clap::Parser;
use log::LevelFilter;
use std::{convert::identity, error::Error, path::PathBuf, str::FromStr, sync::LazyLock};

mod cache;
mod database;
mod domain;
mod web;

use crate::database::DatabasePool;

#[derive(Parser)]
struct Args {
    /// Port of the Application
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
    /// The Host Interface
    #[arg(long, default_value_t = {"127.0.0.1".to_string()})]
    host: String,
    /// Directory to serve. Needs to contain public, hidden and private subdirs
    #[arg(long, required = true)]
    content_dir: PathBuf,
    /// Log Level
    #[arg(long, default_value_t = {"Info".to_string()})]
    log_level: String,
    /// Postgres Database Url to connect to
    #[arg(short, long)]
    database_url: Option<String>,
    /// Oauth Url to authorize against
    #[arg(short, long)]
    auth_url: String,
    /// Oauth Url to get tokens from
    #[arg(short, long)]
    token_url: String,
    /// Oauth Url to get user info from
    #[arg(short, long)]
    user_info: String,
    /// How many web workers to spawn. Default is the number of CPU cores
    #[arg(short = 'j', long)]
    workers: Option<usize>,
    /// Cors origin to allow request from. Can be specified multiple times
    #[arg(long)]
    cors_allowed_origin: Vec<String>,
    /// Define an ical calender to fetch, formatted like name=calendar-url. The calendar will be
    /// available under /api/calendar/<name>. Can be specified multiple times.
    #[arg(short = 'C', long = "calendar", value_parser = parse_key_val::<String, String>)]
    calendars: Vec<(String, String)>,
    /// Define the max file size for uploads in bytes
    #[arg(long, default_value_t = 1024 * 1024 * 10)]
    max_file_size: u64,
    /// Define the datadir for the uploads
    #[arg(long)]
    data_dir: PathBuf,
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
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

static UPLOAD_DIR: LazyLock<PathBuf> = LazyLock::new(|| ARGS.data_dir.join("uploads/attachments/"));

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
