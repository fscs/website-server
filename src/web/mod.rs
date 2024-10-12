use std::future::Future;
use std::path::{Component, PathBuf};
use std::pin::Pin;

use actix_cors::Cors;
use actix_files::NamedFile;
use actix_web::body::BoxBody;
use actix_web::dev::{Payload, ServiceResponse};
use actix_web::error::ErrorNotFound;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::http::StatusCode;
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers, Logger};
use actix_web::web::{scope, Data};
use actix_web::{
    get, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder, ResponseError,
};
use serde::Serialize;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipauto::utoipauto;

pub(crate) mod antrag;
pub(crate) mod auth;
pub(crate) mod calendar;
pub(crate) mod door_state;
pub(crate) mod persons;
pub(crate) mod roles;
pub(crate) mod sitzungen;

use crate::database::{DatabaseConnection, DatabasePool, DatabaseTransaction};
use crate::domain::Error;
use crate::{ARGS, CONTENT_DIR};
use auth::{oauth_client, AuthMiddle, User};

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
    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allowed_origin_fn(|o, _| {
                        let bytes = o.as_bytes();

                        bytes.ends_with(b".hhu-fscs.de")
                    })
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials(),
            )
            .wrap(AuthMiddle)
            .wrap(Logger::default())
            .app_data(Data::new(database.clone()))
            .app_data(Data::new(oauth_client()))
            .service(auth::service("/auth"))
            // /api/docs needs to be before /api
            .service(SwaggerUi::new("/api/docs/{_:.*}").url("/api/openapi.json", ApiDoc::openapi()))
            .service(
                scope("/api")
                    .service(calendar::service("/calendar"))
                    .service(persons::service("/persons"))
                    .service(roles::service("/roles"))
                    .service(antrag::service("/anträge"))
                    .service(door_state::service("/doorstate"))
                    .service(sitzungen::service("/sitzungen")),
            )
            .service(serve_files)
    });

    let server = if let Some(workers) = ARGS.workers {
        server.workers(workers)
    } else {
        server
    };

    server.bind((ARGS.host.as_str(), ARGS.port))?.run().await?;

    Ok(())
}

#[get(
    "/{filename:.*}",
    wrap = "ErrorHandlers::new().handler(StatusCode::NOT_FOUND, file_not_found)"
)]
async fn serve_files(
    req: HttpRequest,
    user: Option<User>,
) -> Result<impl Responder, actix_web::Error> {
    // decide what the user gets to see
    let base_dir = match user {
        Some(user) => match user.is_rat() {
            true => CONTENT_DIR.protected.as_path(),
            false => CONTENT_DIR.hidden.as_path(),
        },
        None => CONTENT_DIR.public.as_path(),
    };

    let sub_path: PathBuf = req.match_info().query("filename").parse().unwrap();

    // validate that the sub_path doesnt go backwards
    for component in sub_path.components() {
        if matches!(component, Component::ParentDir | Component::Prefix(_)) {
            return Err(ErrorNotFound("not found"));
        }
    }

    let path = base_dir.join(sub_path.as_path());
    let actual_path = if path.is_dir() {
        path.join("index.html")
    } else {
        path
    };

    let Ok(file) = NamedFile::open(actual_path.as_path()) else {
        return Err(ErrorNotFound("not found"));
    };

    // configure headers for cache control
    //
    // we enforce html to always be revalidated. we assume all of our assets to be fingerprinted,
    // so those can be cached
    let must_revalidate = *file.content_type() == mime::TEXT_HTML;

    // we dont want to set Last-Modified on responses. since we're nix-people, the date will always
    // be 1970-01-01 which is kind of unnessecary to set
    //
    // ETag should only be set if we want the browser to revalidate
    let res = if must_revalidate {
        file.use_last_modified(false)
            .customize()
            .insert_header(CacheControl(vec![
                CacheDirective::Private,
                CacheDirective::MustRevalidate,
                CacheDirective::MaxAge(0),
            ]))
            .respond_to(&req)
    } else {
        file.use_last_modified(false)
            .use_etag(false)
            .customize()
            .insert_header(CacheControl(vec![
                CacheDirective::Extension("immutable".to_string(), None),
                CacheDirective::MaxAge(31_536_000),
            ]))
            .respond_to(&req)
    };

    Ok(res)
}

fn file_not_found(
    srv_res: ServiceResponse<BoxBody>,
) -> actix_web::Result<ErrorHandlerResponse<BoxBody>> {
    let req = srv_res.request();
    let path = ARGS.content_dir.join("de/404.html");

    let file = NamedFile::open(path).unwrap();

    let http_res = file
        .use_last_modified(false)
        .customize()
        .insert_header(CacheControl(vec![
            CacheDirective::Private,
            CacheDirective::MustRevalidate,
            CacheDirective::MaxAge(0),
        ]))
        .respond_to(req);

    Ok(ErrorHandlerResponse::Response(
        srv_res.into_response(http_res),
    ))
}
