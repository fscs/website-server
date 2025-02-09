use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;

use actix_cors::Cors;
use actix_files::NamedFile;
use actix_http::StatusCode;
use actix_web::body::BoxBody;
use actix_web::dev::Payload;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{
    get, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder, ResponseError,
};
use serde::Serialize;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipauto::utoipauto;

pub(crate) mod api;
pub(crate) mod auth;

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
    let database_data = Data::new(database);
    let calendar_data = Data::new(api::calendar::app_data());
    
    let server = HttpServer::new(move || {
        let mut cors = Cors::default()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();
        
        for allowed in &ARGS.cors_allowed_origin {
           cors = cors.allowed_origin(allowed.as_str()) 
        }

        App::new()
            .wrap(AuthMiddle)
            .wrap(cors)
            .wrap(Logger::default())
            .app_data(database_data.clone())
            .app_data(calendar_data.clone())
            .app_data(Data::new(oauth_client()))
            .service(auth::service())
            // /api/docs needs to be before /api
            .service(SwaggerUi::new("/api/docs/{_:.*}").url("/api/openapi.json", ApiDoc::openapi()))
            .service(api::service())
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

#[get("/{filename:.*}")]
async fn serve_files(req: HttpRequest, user: Option<User>) -> HttpResponse<BoxBody> {
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
            return err_not_found(base_dir, req).await;
        }
    }

    let path = base_dir.join(sub_path.as_path());
    let actual_path = if path.is_dir() {
        path.join("index.html")
    } else {
        path
    };

    let Ok(file) = NamedFile::open_async(actual_path.as_path()).await else {
        return err_not_found(base_dir, req).await;
    };

    // configure headers for cache control
    //
    // we enforce html to always be revalidated. we assume all of our assets to be fingerprinted,
    // so those can be cached
    let must_revalidate = *file.content_type() == mime::TEXT_HTML;

    // we dont want to set Last-Modified on responses. since the content will live in the nix store
    // anyway and the date will always be 1970-01-01 which is kind of unnessecary to set
    //
    // ETag should only be set if we want the browser to revalidate
    if must_revalidate {
        file.use_last_modified(false)
            .customize()
            .insert_header(CacheControl(vec![
                CacheDirective::Private,
                CacheDirective::MustRevalidate,
                CacheDirective::MaxAge(0),
            ]))
            .respond_to(&req)
            .map_into_boxed_body()
    } else {
        file.use_last_modified(false)
            .use_etag(false)
            .customize()
            .insert_header(CacheControl(vec![
                CacheDirective::Extension("immutable".to_string(), None),
                CacheDirective::MaxAge(31_536_000),
            ]))
            .respond_to(&req)
            .map_into_boxed_body()
    }
}

async fn err_not_found(base_dir: &Path, req: HttpRequest) -> HttpResponse<BoxBody> {
    let path = base_dir.join("de").join("404.html");

    NamedFile::open_async(path)
        .await
        .map(|f| {
            f.customize()
                .with_status(StatusCode::NOT_FOUND)
                .respond_to(&req)
                .map_into_boxed_body()
        })
        .unwrap_or_else(|_| HttpResponse::NotFound().body("<h1>Not found</h1>"))
}
