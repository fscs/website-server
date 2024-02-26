mod web;
use actix_files as fs;
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {

    Ok(HttpServer::new(move || App::new()
        .service(web::calendar::get_events)
        .service(fs::Files::new("/", "static/")))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await?)
}