use crate::database::{DatabasePool, DatabaseTransaction};
use crate::{domain, get_base_dir, web, ARGS};
use actix_files as fs;
use actix_web::body::BoxBody;
use actix_web::dev::{Payload, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::web::Data;
use actix_web::{App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Error;
use serde::Serialize;
use serde_json::json;
use std::fs::File;
use std::future::Future;
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use utoipa::openapi::Server;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

pub(crate) mod abmeldungen;
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

impl FromRequest for DatabaseTransaction<'static> {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

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
            calendar::get_events,
            calendar::get_branchen_events,
            abmeldungen::get_abmeldungen,
            abmeldungen::put_person_abmeldung,
            abmeldungen::update_person_abmeldung,
            abmeldungen::delete_person_abmeldung,
            topmanager::antrag::create_antrag,
            topmanager::antrag::update_antrag,
            topmanager::antrag::delete_antrag,
            topmanager::antrag::get_anträge,
            topmanager::antrag::get_antrag,
            topmanager::sitzungen::get_sitzungen,
            topmanager::sitzungen::create_sitzung,
            topmanager::sitzungen::create_top,
            topmanager::sitzungen::tops_by_sitzung,
            topmanager::sitzungen::get_next_sitzung,
            topmanager::anträge_by_top,
            topmanager::get_current_tops_with_anträge,
            topmanager::anträge_by_sitzung
        ),
        components(schemas(
            doorstate::CreateDoorStateParams,
            domain::Doorstate,
            person::CreatePersonRoleParams,
            person::GetPersonsByRoleParams,
            person::CreatePersonParams,
            domain::Person,
            calendar::CalendarEvent,
            domain::Abmeldung,
            abmeldungen::CreatePersonAbmeldungParams,
            topmanager::antrag::CreateAntragParams,
            topmanager::antrag::UpdateAntragParams,
            topmanager::antrag::DeleteAntragParams,
            domain::Antrag,
            domain::Top,
            domain::PersonRoleMapping,
            domain::Sitzung,
            domain::Antragsstellende,
            topmanager::TopWithAnträge,
            topmanager::Person,
            topmanager::CreateTopParams,
            topmanager::sitzungen::CreateSitzungParams
        ))
    )]
    struct ApiDoc;

    let openapi = ApiDoc::openapi();
    let config = || {
        json!({
            "hideDownloadButton": true,
            "expandResponses": "all",
            "requiredPropsFirst": true,
            "theme": {
                "colors": {
                    "primary": {
                        "main": "#6EC5AB",
                        "light": "#6EC5AB",
                        "dark": "#6EC5AB",
                    },

                },
                "typography": {
                    "fontSize": "15px",
                    "lineHeight": "1.5",
                    "code": {
                        "code": "#ff00ff",
                        "backgroundColor": "#00ffff",
                    },
                },
                "codeBlock": {
                    "backgroundColor": "#1C212C",
                    "tokens": {
                        "comment": {
                            "color": "#8C8E94",
                        },
                        "keyword": {
                            "color": "#E6895B",
                        },
                        "number": {
                            "color": "##9BCF6E",
                        },
                        "string": {
                            "color": "#E6895B",
                        },
                    },
                },
                "menu": {
                    "backgroundColor": "#1C212C",
                    "textColor": "#FFFFFF",
                },
                "rightPanel": {
                    "backgroundColor": "#1C212C",
                    "textColor": "#FFFFFF",
                },


            }
        })
    };
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found))
            .service(web::calendar::service("/api/calendar"))
            .service(topmanager::service("/api/topmanager"))
            .service(doorstate::service("/api/doorstate"))
            .service(person::service("/api/person"))
            .service(abmeldungen::service("/api/abmeldungen"))
            .service(Redoc::with_url_and_config(
                "/redoc/",
                openapi.clone(),
                config,
            ))
            .service(
                SwaggerUi::new("/api/docs/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
            .service(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
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
    if res.headers().get("content-type")
        != Some(&actix_web::http::header::HeaderValue::from_static(
            "text/html",
        ))
    {
        return Ok(ErrorHandlerResponse::Response(res.map_into_left_body()));
    };

    let (req, res) = res.into_parts();
    let path = PathBuf::from_str(format!("/{}/static/404.html", get_base_dir().unwrap()).as_str())
        .unwrap();
    let mut file = File::open(path).unwrap();

    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    let res = res.set_body(content);

    let res = ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res))
}
