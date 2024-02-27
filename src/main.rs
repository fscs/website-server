mod web;
mod cache;

use actix_files as fs;
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let current_dir = std::env::current_exe()
        .unwrap()
        .as_path()
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    Ok(HttpServer::new(move || {
        App::new()
            .service(web::calendar::get_branchen_events)
            .service(web::calendar::get_events).service(
            fs::Files::new("/", &(current_dir.clone() + "/static/")).index_file("index.html"),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?)
}
