use crate::database::{DatabasePool, DatabaseTransaction};
use crate::web::auth::AuthMiddle;
use crate::{domain, web, ARGS};
use actix_files as fs;
use actix_web::body::BoxBody;
use actix_web::dev::{Payload, Service, ServiceRequest, ServiceResponse};
use actix_web::error::{ErrorNotFound, ErrorUnauthorized};
use actix_web::guard::GuardContext;
use actix_web::http::header::{self, ContentDisposition, ContentType, DispositionType};
use actix_web::http::StatusCode;
use actix_web::middleware::{self, ErrorHandlerResponse, ErrorHandlers};
use actix_web::web::{Data, Redirect};
use actix_web::{
    get, guard, App, FromRequest, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer,
    Responder,
};
use anyhow::Error;
use futures_util::future::Either;
use futures_util::FutureExt;
use reqwest::Body;
use serde::Serialize;

use std::fs::File;
use std::future::Future;
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;

use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;

use self::auth::{oauth_client, OauthClient, User};
use utoipa_swagger_ui::{oauth, SwaggerUi};

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
                Err(e) => RestStatus::Error(anyhow::Error::from(e)),
            },
            Err(e) => RestStatus::Error(anyhow::Error::from(e)),
        }
    }

    fn ok_from_result<T: Serialize>(result: anyhow::Result<T>) -> RestStatus {
        match result {
            Ok(antrag) => match serde_json::to_value(antrag) {
                Ok(value) => RestStatus::Ok(value),
                Err(e) => RestStatus::Error(anyhow::Error::from(e)),
            },
            Err(e) => RestStatus::Error(anyhow::Error::from(e)),
        }
    }

    fn ok_or_not_found_from_result<T: Serialize>(result: anyhow::Result<Option<T>>) -> RestStatus {
        match result {
            Ok(Some(antrag)) => match serde_json::to_value(antrag) {
                Ok(value) => RestStatus::Ok(value),
                Err(e) => RestStatus::Error(anyhow::Error::from(e)),
            },
            Ok(None) => RestStatus::NotFound,
            Err(e) => RestStatus::Error(anyhow::Error::from(e)),
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
            topmanager::antrag::CreateAntragParams,
            topmanager::antrag::UpdateAntragParams,
            topmanager::antrag::DeleteAntragParams,
            topmanager::antrag::CreateAntragTopMappingParams,
            domain::Antrag,
            domain::Top,
            domain::PersonRoleMapping,
            domain::Sitzung,
            domain::Antragsstellende,
            topmanager::TopWithAnträge,
            topmanager::Person,
            topmanager::CreateTopParams,
            topmanager::sitzungen::CreateSitzungParams,
            topmanager::sitzungen::DeleteSitzungParams,
            topmanager::sitzungen::UpdateSitzungParams,
            topmanager::sitzungen::UpdateTopParams,
            topmanager::sitzungen::DeleteTopParams,
        ))
    )]
    struct ApiDoc;

    let openapi = ApiDoc::openapi();

    Ok(HttpServer::new(move || {
        App::new()
            .wrap(
                ErrorHandlers::new()
                    .handler(StatusCode::NOT_FOUND, not_found)
                    .handler(StatusCode::UNAUTHORIZED, web::auth::not_authorized),
            )
            .wrap(AuthMiddle)
            .service(web::calendar::service("/api/calendar"))
            .service(topmanager::service("/api/topmanager"))
            .service(doorstate::service("/api/doorstate"))
            .service(auth::service("/auth"))
            .service(person::service("/api/person"))
            .service(abmeldungen::service("/api/abmeldungen"))
            .service(
                SwaggerUi::new("/api/docs/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
            .service(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
            .service(serve_files)
            .app_data(Data::new(database.clone()))
            .app_data(Data::new(oauth_client()))
    })
    .bind((ARGS.host.as_str(), ARGS.port))?
    .run()
    .await?)
}

#[get("/{filename:.*}")]
async fn serve_files(
    req: HttpRequest,
    user: Option<User>,
) -> Result<HttpResponse, actix_web::Error> {
    let base_dir_raw = match user {
        Some(user) => match user.is_rat() {
            true => ARGS.private_content_dir.as_path(),
            false => ARGS.hidden_content_dir.as_path(),
        },
        None => ARGS.content_dir.as_path(),
    };

    let base_dir = base_dir_raw
        .canonicalize()
        .map_err(|_| ErrorNotFound("not found"))?;

    let mut sub_path: std::path::PathBuf = req.match_info().query("filename").parse().unwrap();

    let path = base_dir
        .join(sub_path)
        .canonicalize()
        .map_err(|_| ErrorNotFound("not found"))?;

    if !path.exists() || !path.starts_with(base_dir.as_path()) {
        return Err(ErrorNotFound("not found").into());
    }

    let file = if path.is_dir() {
        fs::NamedFile::open(path.join("index.html"))?
    } else {
        fs::NamedFile::open(path)?
    };

    Ok(file.into_response(&req))
}

fn not_found<B>(
    res: actix_web::dev::ServiceResponse<B>,
) -> actix_web::Result<actix_web::middleware::ErrorHandlerResponse<B>> {
    let (req, res) = res.into_parts();
    let path = ARGS.content_dir.join("de/404.html");

    let mut file = File::open(path).unwrap();

    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    let status = res.status();

    let new_response = HttpResponseBuilder::new(status)
        .insert_header(ContentType::html())
        .body(content);

    Ok(ErrorHandlerResponse::Response(
        ServiceResponse::new(req, new_response).map_into_right_body(),
    ))
}
