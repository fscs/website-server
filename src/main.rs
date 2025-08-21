#![warn(clippy::shadow_unrelated)]

use async_std::{fs, path::PathBuf, sync::RwLock};
use clap::Parser;
use domain::templates::TemplatesRepo;
use log::LevelFilter;
use std::{error::Error, str::FromStr, sync::LazyLock};

mod cache;
mod database;
mod domain;
mod web;

use crate::{database::DatabasePool, domain::Capability};

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

    /// Oauth Source Name
    #[arg(short, long, requires_all = ["auth_url", "token_url", "user_info"])]
    oauth_source_name: Option<String>,
    /// Oauth Url to authorize against
    #[arg(short, long, requires_all = ["oauth_source_name", "token_url", "user_info"])]
    auth_url: Option<String>,
    /// Oauth Url to get tokens from
    #[arg(short, long, requires_all = ["auth_url", "oauth_source_name", "user_info"])]
    token_url: Option<String>,
    /// Oauth Url to get user info from
    #[arg(short, long, requires_all = ["auth_url", "token_url", "oauth_source_name"])]
    user_info: Option<String>,
    /// Specifiy a group and grant it capabilities.. Parameter should be formatted like
    /// 'GroupName=CapName[,CapName]'
    #[arg(long = "group", value_parser = parse_key_val::<String, String>)]
    groups: Vec<(String, String)>,
    /// Specify Capabilities to be granted to Users that arent logged in
    #[arg(long = "default-capability")]
    default_capabilities: Vec<Capability>,

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
    max_file_size: usize,
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

static TEMPLATE_ENGINE: LazyLock<RwLock<upon::Engine>> =
    LazyLock::new(|| RwLock::new(upon::Engine::new()));

#[actix_web::main]
async fn main() -> domain::Result<()> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(
            LevelFilter::from_str(&ARGS.log_level)
                .map_err(|e| domain::Error::Message(format!("{:?}", e)))?,
        )
        .init();

    fs::create_dir_all(UPLOAD_DIR.as_path()).await?;

    let database_url = ARGS
        .database_url
        .clone()
        .or(std::env::var("DATABASE_URL").ok())
        .unwrap_or("postgres://postgres:postgres@localhost/postgres".to_string());

    let database = DatabasePool::new(&database_url)
        .await
        .map_err(|e| domain::Error::Message(format!("failed to aquire database: {:?}", e)))?;

    let mut transaction = database.start_transaction().await?;
    sqlx::migrate!()
        .run(&mut *transaction)
        .await
        .map_err(|e| domain::Error::Message(format!("error while running migrations: {:?}", e)))?;
    transaction.commit().await?;

    let templates = database.aquire().await?.templates().await?;

    for template in templates {
        TEMPLATE_ENGINE
            .write()
            .await
            .add_template(template.name, template.inhalt)?;
    }

    web::start_server(database).await
}
