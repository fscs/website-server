use crate::database::{DatabasePool, DatabaseTransaction};
use crate::web::auth::AuthMiddle;
use crate::{domain, web, ARGS};
use actix_files::NamedFile;
use actix_web::body::BoxBody;
use actix_web::dev::{Payload, ServiceResponse};
use actix_web::error::ErrorNotFound;
use actix_web::http::header::{CacheControl, CacheDirective, ContentType};
use actix_web::http::{header, StatusCode};
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::web::{scope, Data};
use actix_web::{
    get, App, FromRequest, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Responder,
};
use anyhow::Error;
use serde::Serialize;

use std::fs::File;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;

use utoipa::OpenApi;

use self::auth::{oauth_client, User};
use utoipa_swagger_ui::SwaggerUi;

pub(crate) mod abmeldungen;
pub(crate) mod auth;
pub(crate) mod calendar;
pub(crate) mod doorstate;
pub(crate) mod person;
pub(crate) mod topmanager;

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

impl DatabaseTransaction<'_> {
    pub(crate) async fn rest_ok<T: Serialize>(self, result: anyhow::Result<T>) -> RestStatus {
        match result {
            Ok(r) => match self.commit().await {
                Ok(()) => match serde_json::to_value(r) {
                    Ok(v) => RestStatus::Ok(v),
                    Err(e) => RestStatus::Error(e.into()),
                },
                Err(e) => RestStatus::Error(e),
            },
            Err(e) => RestStatus::Error(e),
        }
    }

    pub(crate) async fn rest_created<T: Serialize>(self, result: anyhow::Result<T>) -> RestStatus {
        match result {
            Ok(r) => match self.commit().await {
                Ok(()) => match serde_json::to_value(r) {
                    Ok(v) => RestStatus::Created(v),
                    Err(e) => RestStatus::Error(e.into()),
                },
                Err(e) => RestStatus::Error(e),
            },
            Err(e) => RestStatus::Error(e),
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

pub async fn start_server(database: DatabasePool) -> Result<(), Error> {
    #[derive(OpenApi)]
    #[openapi(
        info(
            title = "FSCS API",
            description = "Our API to manage the FSCS System",
            contact(name = "FSCS", email = "fscs@hhu.de", url = "https://new.hhu-fscs.de"),
            version = "1.0.0"
        ),
        paths(
            doorstate::put_doorstate,
            doorstate::get_doorstate,
            person::put_person_role,
            person::get_persons,
            person::get_person_by_role,
            person::create_person,
            person::patch_person,
            person::delete_person,
            person::update_person_role,
            person::delete_person_role,
            calendar::get_events,
            calendar::get_branchen_events,
            abmeldungen::get_abmeldungen,
            abmeldungen::put_person_abmeldung,
            abmeldungen::update_person_abmeldung,
            abmeldungen::delete_person_abmeldung,
            abmeldungen::get_abmeldungen_next_sitzungen,
            abmeldungen::get_abmeldungen_between,
            topmanager::antrag::create_antrag,
            topmanager::antrag::create_antrag_for_top,
            topmanager::antrag::update_antrag,
            topmanager::antrag::delete_antrag,
            topmanager::antrag::get_anträge,
            topmanager::antrag::get_antrag,
            topmanager::antrag::put_antrag_top_mapping,
            topmanager::antrag::delete_antrag_top_mapping,
            topmanager::sitzungen::get_sitzungen,
            topmanager::sitzungen::get_sitzung,
            topmanager::sitzungen::get_sitzung_by_date,
            topmanager::sitzungen::create_sitzung,
            topmanager::sitzungen::create_top,
            topmanager::sitzungen::tops_by_sitzung,
            topmanager::sitzungen::get_next_sitzung,
            topmanager::sitzungen::delete_sitzung,
            topmanager::sitzungen::update_sitzung,
            topmanager::sitzungen::update_top,
            topmanager::sitzungen::delete_top,
            topmanager::anträge_by_top,
            topmanager::get_current_tops_with_anträge,
            topmanager::get_tops_by_date_with_anträge,
            topmanager::anträge_by_sitzung,
            topmanager::get_top,
        ),
        components(schemas(
            doorstate::CreateDoorStateParams,
            domain::Doorstate,
            person::CreatePersonRoleParams,
            person::GetPersonsByRoleParams,
            person::CreatePersonParams,
            person::UpdatePersonParams,
            person::DeletePersonParams,
            person::UpdatePersonRoleParams,
            person::DeletePersonRoleParams,
            domain::Person,
            calendar::CalendarEvent,
            domain::Abmeldung,
            abmeldungen::CreatePersonAbmeldungParams,
            abmeldungen::GetAbmeldungBetweenParams,
            topmanager::antrag::CreateAntragParams,
            topmanager::antrag::UpdateAntragParams,
            topmanager::antrag::DeleteAntragParams,
            topmanager::antrag::CreateAntragTopMappingParams,
            domain::Antrag,
            domain::Top,
            domain::PersonRoleMapping,
            domain::SitzungType,
            domain::Sitzung,
            domain::Antragsstellende,
            topmanager::TopWithAnträge,
            topmanager::Person,
            topmanager::CreateTopParams,
            topmanager::GetTopsByDateParams,
            topmanager::sitzungen::CreateSitzungParams,
            topmanager::sitzungen::GetSitzungByDateParams,
            topmanager::sitzungen::DeleteSitzungParams,
            topmanager::sitzungen::UpdateSitzungParams,
            topmanager::sitzungen::UpdateTopParams,
            topmanager::sitzungen::DeleteTopParams,
        ))
    )]
    struct ApiDoc;

    HttpServer::new(move || {
        App::new()
            .wrap(ErrorHandlers::new().handler(StatusCode::UNAUTHORIZED, web::auth::not_authorized))
            .wrap(AuthMiddle)
            .app_data(Data::new(database.clone()))
            .app_data(Data::new(oauth_client()))
            .service(auth::service("/auth"))
            // /api/docs needs to be before /api
            .service(SwaggerUi::new("/api/docs/{_:.*}").url("/api/openapi.json", ApiDoc::openapi()))
            .service(
                scope("/api")
                    .service(calendar::service("/calendar"))
                    .service(topmanager::service("/topmanager"))
                    .service(doorstate::service("/doorstate"))
                    .service(person::service("/person"))
                    .service(abmeldungen::service("/abmeldungen")),
            )
            .service(serve_files)
    })
    .bind((ARGS.host.as_str(), ARGS.port))?
    .run()
    .await?;

    Ok(())
}

#[get("/{filename:.*}", wrap = "ErrorHandlers::new().handler(StatusCode::NOT_FOUND, file_not_found)")]
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
                CacheDirective::MaxAge(31536000),
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
        .respond_to(&req);

    Ok(ErrorHandlerResponse::Response(
        srv_res.into_response(http_res),
    ))
}
