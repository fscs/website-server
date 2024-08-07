use actix_web::{delete, web};
use actix_web::{get, patch, put, Responder, Scope};
use chrono::NaiveDate;
use serde::Deserialize;
use sqlx::types::chrono;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::database::DatabaseTransaction;
use crate::domain::AbmeldungRepo;

use super::auth::User;

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(put_person_abmeldung)
        .service(get_abmeldungen)
        .service(get_abmeldungen_between)
        .service(get_abmeldungen_next_sitzungen)
        .service(delete_person_abmeldung)
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct CreatePersonAbmeldungParams {
    pub person_id: Uuid,
    pub anfangsdatum: chrono::NaiveDate,
    pub ablaufdatum: chrono::NaiveDate,
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct GetAbmeldungBetweenParams {
    pub start: NaiveDate,
    pub end: NaiveDate,
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
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<CreatePersonAbmeldungParams>,
) -> impl Responder {
    let result = transaction
        .add_person_abmeldung(params.person_id, params.anfangsdatum, params.ablaufdatum)
        .await;

    transaction.rest_ok(result).await
}

#[utoipa::path(
    path = "/api/abmeldungen/",
    responses(
        (status = 200, description = "Success", body = Vec<Abmeldung>),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/")]
async fn get_abmeldungen(mut transaction: DatabaseTransaction<'_>, _user: User) -> impl Responder {
    let result = transaction.get_abmeldungen().await;

    transaction.rest_ok(result).await
}

#[utoipa::path(
    path = "/api/abmeldungen/next_sitzung/",
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/next_sitzung/")]
async fn get_abmeldungen_next_sitzungen(
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    let now = chrono::Utc::now();
    let date = now.date_naive();
    let result = transaction.get_abmeldungen_between(&date, &date).await;

    transaction.rest_ok(result).await
}

#[utoipa::path(
    path = "/api/abmeldungen/between/",
    request_body = GetAbmeldungBetweenParams,
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/between/")]
async fn get_abmeldungen_between(
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<GetAbmeldungBetweenParams>,
) -> impl Responder {
    let result = transaction
        .get_abmeldungen_between(&params.start, &params.end)
        .await;

    transaction.rest_ok(result).await
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
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<CreatePersonAbmeldungParams>,
) -> impl Responder {
    let result = transaction
        .update_person_abmeldung(params.person_id, params.anfangsdatum, params.ablaufdatum)
        .await;

    transaction.rest_ok(result).await
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
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<CreatePersonAbmeldungParams>,
) -> impl Responder {
    let result = transaction
        .delete_person_abmeldung(params.person_id, params.anfangsdatum, params.ablaufdatum)
        .await;

    transaction.rest_ok(result).await
}
