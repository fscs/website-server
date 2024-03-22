use std::fs::File;
use crate::database::{DatabasePool, DatabaseTransaction};
use actix_web::dev::{Payload, ServiceResponse};
use actix_web::{App, FromRequest, HttpRequest, HttpServer};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use anyhow::Error;
use actix_files as fs;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use std::str::FromStr;
use std::io::Read;
use crate::{ARGS, get_base_dir, web};

pub(crate) mod calendar;
pub(crate) mod topmanager;

impl FromRequest for DatabaseTransaction<'static> {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            if let Some(pool) = req.app_data::<DatabasePool>() {
                match pool.start_transaction().await {
                    Ok(transaction) => Ok(transaction),
                    Err(err) => {
                        log::debug!("{:?}", err);
                        Err(actix_web::error::ErrorInternalServerError(
                            "Could not access Database",
                        ))
                    }
                }
            } else {
                log::debug!("Failed to extract the DatabasePool");
                Err(actix_web::error::ErrorInternalServerError(
                    "Requested application data is not configured correctly",
                ))
            }
        })
    }
}

pub async fn start_server(dir: String, database: DatabasePool) -> Result<(), Error> {
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found))
            .service(web::calendar::service("/api/calendar"))
            .service(topmanager::service("/api/topmanager"))
            .service(fs::Files::new("/", dir.clone() + "/static/").index_file("index.html"))
            .app_data(Data::new(database.clone()))
    })
        .bind((ARGS.host.as_str(), ARGS.port))?
        .run()
        .await?)
}

fn not_found<B>(
    res: actix_web::dev::ServiceResponse<B>,
) -> actix_web::Result<actix_web::middleware::ErrorHandlerResponse<B>> {
    if res.headers().get("content-type") != Some(&actix_web::http::header::HeaderValue::from_static("text/html")) {
        return Ok(ErrorHandlerResponse::Response(res.map_into_left_body()));
    };

    let (req, res) = res.into_parts();
    let path = PathBuf::from_str(format!("/{}/static/404.html", get_base_dir().unwrap()).as_str()).unwrap();
    let mut file = File::open(path).unwrap();

    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    let res = res.set_body(content);

    let res = ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res))
}
