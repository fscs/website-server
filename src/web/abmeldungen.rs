use actix_web::{delete, web};
use actix_web::{get, patch, put, web::Data, Responder, Scope};
use serde::Deserialize;
use sqlx::types::chrono;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{database::DatabasePool, domain::AbmeldungRepo, web::RestStatus};

use super::auth::User;

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(put_person_abmeldung)
        .service(get_abmeldungen)
        .service(get_abmeldungen_next_sitzungen)
        .service(delete_person_abmeldung)
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct CreatePersonAbmeldungParams {
    pub person_id: Uuid,
    pub anfangsdatum: chrono::NaiveDate,
    pub ablaufdatum: chrono::NaiveDate,
}

#[utoipa::path(
    path = "/api/abmeldungen/",
    request_body = CreatePersonAbmeldungParams,
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/")]
async fn put_person_abmeldung(
    _user: User,
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonAbmeldungParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction
                    .add_person_abmeldung(params.person_id, params.anfangsdatum, params.ablaufdatum)
                    .await?;
                Ok((person, transaction))
            }
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[utoipa::path(
    path = "/api/abmeldungen/",
    responses(
        (status = 200, description = "Success", body = Vec<Abmeldung>),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/")]
async fn get_abmeldungen(db: Data<DatabasePool>) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| async move {
            let person = transaction.get_abmeldungen().await?;
            Ok((person, transaction))
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[utoipa::path(
    path = "/api/abmeldungen/next_sitzung/",
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/next_sitzung/")]
async fn get_abmeldungen_next_sitzungen(db: Data<DatabasePool>) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| async move {
            let person = transaction.get_abmeldungen_next_sitzung().await?;
            Ok((person, transaction))
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[utoipa::path(
    path = "/api/abmeldungen/",
    request_body = CreatePersonAbmeldungParams,
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 400, description = "Bad Request"),
    )
)]
#[patch("/")]
async fn update_person_abmeldung(
    _user: User,
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonAbmeldungParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction
                    .update_person_abmeldung(
                        params.person_id,
                        params.anfangsdatum,
                        params.ablaufdatum,
                    )
                    .await?;
                Ok((person, transaction))
            }
        })
        .await;
    RestStatus::ok_from_result(result)
}

#[utoipa::path(
    path = "/api/abmeldungen/",
    request_body = CreatePersonAbmeldungParams,
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 400, description = "Bad Request"),
    )
)]
#[delete("/")]
async fn delete_person_abmeldung(
    _user: User,
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonAbmeldungParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                transaction
                    .delete_person_abmeldung(
                        params.person_id,
                        params.anfangsdatum,
                        params.ablaufdatum,
                    )
                    .await?;
                Ok(((), transaction))
            }
        })
        .await;
    RestStatus::ok_from_result(result)
}
