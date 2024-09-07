use std::future::Future;
use std::path::{Component, PathBuf};
use std::pin::Pin;

use actix_files::NamedFile;
use actix_web::body::BoxBody;
use actix_web::dev::{Payload, ServiceResponse};
use actix_web::error::ErrorNotFound;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::http::StatusCode;
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::web::{scope, Data};
use actix_web::{get, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Error;
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

use crate::database::{DatabasePool, DatabaseTransaction};
use crate::ARGS;
use auth::{oauth_client, AuthMiddle, User};

pub(super) enum RestStatus {
    Ok(serde_json::Value),
    Created(serde_json::Value),
    NotFound,
    Error(anyhow::Error),
}

impl RestStatus {
    fn created_from_result<T: Serialize>(result: anyhow::Result<T>) -> RestStatus {
        match result {
            Ok(antrag) => match serde_json::to_value(antrag) {
                Ok(value) => RestStatus::Created(value),
                Err(e) => RestStatus::Error(e.into()),
            },
            Err(e) => RestStatus::Error(e),
        }
    }

    fn ok_from_result<T: Serialize>(result: anyhow::Result<T>) -> RestStatus {
        match result {
            Ok(antrag) => match serde_json::to_value(antrag) {
                Ok(value) => RestStatus::Ok(value),
                Err(e) => RestStatus::Error(e.into()),
            },
            Err(e) => RestStatus::Error(e),
        }
    }

    fn ok_or_not_found_from_result<T: Serialize>(result: anyhow::Result<Option<T>>) -> RestStatus {
        match result {
            Ok(Some(antrag)) => match serde_json::to_value(antrag) {
                Ok(value) => RestStatus::Ok(value),
                Err(e) => RestStatus::Error(e.into()),
            },
            Ok(None) => RestStatus::NotFound,
            Err(e) => RestStatus::Error(e),
        }
    }
}

impl Responder for RestStatus {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            RestStatus::Ok(value) => HttpResponse::Ok().json(value),
            RestStatus::Created(value) => {
                log::debug!("Created: {:?}", value.as_str());
                HttpResponse::Created().json(value)
            }
            RestStatus::NotFound => {
                log::debug!("Resource {} not found", req.path());
                HttpResponse::NotFound().body("Not Found")
            }
            RestStatus::Error(error) => {
                log::error!("{:?}", error);
                HttpResponse::InternalServerError().body("Internal Server Error")
            }
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

#[utoipauto()]
#[derive(OpenApi)]
#[openapi(info(
    title = "FSCS API",
    contact(name = "HHU Fachschaft Informatik", url = "https://fscs.hhu.de"),
))]
struct ApiDoc;

pub async fn start_server(database: DatabasePool) -> Result<(), Error> {
    HttpServer::new(move || {
        App::new()
            .wrap(ErrorHandlers::new().handler(StatusCode::UNAUTHORIZED, auth::not_authorized))
            .wrap(AuthMiddle)
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
    })
    .bind((ARGS.host.as_str(), ARGS.port))?
    .run()
    .await?;

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
            true => ARGS.private_content_dir.as_path(),
            false => ARGS.hidden_content_dir.as_path(),
        },
        None => ARGS.content_dir.as_path(),
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

    let Ok(file) = NamedFile::open(actual_path) else {
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
