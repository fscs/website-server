use std::future::Future;
use std::pin::Pin;

use actix_cors::Cors;
use actix_web::body::BoxBody;
use actix_web::dev::Payload;
use actix_web::middleware::{Logger, TrailingSlash};
use actix_web::web::Data;
use actix_web::{
    App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder, ResponseError,
};
use serde::Serialize;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipauto::utoipauto;

pub(crate) mod api;
pub(crate) mod auth;
pub(crate) mod calendar;
pub(crate) mod files;

use crate::database::{DatabaseConnection, DatabasePool, DatabaseTransaction};
use crate::domain::Error;
use crate::ARGS;
use auth::{oauth_client, AuthMiddle};

pub(super) enum RestStatus<T: Serialize> {
    Success(Option<T>),
    Created(Option<T>),
}

impl ResponseError for Error {}

impl<T: Serialize> Responder for RestStatus<T> {
    type Body = BoxBody;

    fn respond_to(self, _: &HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            RestStatus::Success(value) => match value {
                Some(inner) => HttpResponse::Ok().json(inner),
                None => HttpResponse::NotFound().body("Not Found"),
            },
            RestStatus::Created(value) => match value {
                Some(inner) => HttpResponse::Created().json(inner),
                None => HttpResponse::NotFound().body("Not Found"),
            },
        }
    }
}

impl FromRequest for DatabaseTransaction<'static> {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            if let Some(pool) = req.app_data::<Data<DatabasePool>>() {
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

impl FromRequest for DatabaseConnection {
    type Error = actix_web::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            if let Some(pool) = req.app_data::<Data<DatabasePool>>() {
                match pool.aquire().await {
                    Ok(conn) => Ok(conn),
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

#[utoipauto()]
#[derive(OpenApi)]
#[openapi(info(
    title = "FSCS API",
    contact(name = "HHU Fachschaft Informatik", url = "https://fscs.hhu.de"),
))]
struct ApiDoc;

pub async fn start_server(database: DatabasePool) -> Result<(), Error> {
    let database_data = Data::new(database);
    let calendar_data = Data::new(calendar::CalendarData::new());

    let server = HttpServer::new(move || {
        let mut cors = Cors::default()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        for allowed in &ARGS.cors_allowed_origin {
            cors = cors.allowed_origin(allowed.as_str())
        }

        App::new()
            .wrap(actix_web::middleware::NormalizePath::new(
                TrailingSlash::Trim,
            ))
            .wrap(actix_web::middleware::Compress::default())
            .wrap(AuthMiddle)
            .wrap(cors)
            .wrap(Logger::default())
            .app_data(database_data.clone())
            .app_data(calendar_data.clone())
            .app_data(Data::new(oauth_client()))
            .service(auth::service())
            // /api/docs needs to be before /api
            .service(SwaggerUi::new("/api/docs/{_:.*}").url("/api/openapi.json", ApiDoc::openapi()))
            .service(SwaggerUi::new("/api/docs{_:.*}").url("/api/openapi.json", ApiDoc::openapi()))
            .service(api::service())
            .service(files::service())
    });

    let server = if let Some(workers) = ARGS.workers {
        server.workers(workers)
    } else {
        server
    };

    server.bind((ARGS.host.as_str(), ARGS.port))?.run().await?;

    Ok(())
}
