use actix_web::{get, put, web, Responder, Scope};
use chrono::Utc;
use serde::Deserialize;
use sqlx::types::chrono;
use utoipa::{IntoParams, ToSchema};

use crate::{database::DatabaseTransaction, domain::DoorStateRepo, web::auth::User};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(put_doorstate)
        .service(get_doorstate)
        .service(get_doorstate_history)
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct CreateDoorStateParams {
    pub is_open: bool,
}

#[utoipa::path(
    path = "/api/doorstate/",
    request_body = CreateDoorStateParams,
    responses(
        (status = 200, description = "Success", body = Doorstate),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/")]
async fn put_doorstate(
    _user: User,
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<CreateDoorStateParams>,
) -> impl Responder {
    let now = Utc::now();
    let result = transaction
        .add_doorstate(now.naive_utc(), params.is_open)
        .await;

    transaction.rest_ok(result).await
}

#[utoipa::path(
    path = "/api/doorstate/",
    responses(
        (status = 200, description = "Success", body = Doorstate),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/")]
async fn get_doorstate(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    let now = Utc::now();
    let result = transaction.get_doorstate(now.naive_utc()).await;

    transaction.rest_ok(result).await
}

#[utoipa::path(
    path = "/api/doorstate/history/",
    responses(
        (status = 200, description = "Success", body = Vec<Doorstate>),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/history/")]
async fn get_doorstate_history(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    let result = transaction.get_doorstate_history().await;

    transaction.rest_ok(result).await
}
