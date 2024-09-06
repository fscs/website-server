use actix_web::{
    get, post,
    web::{self, Query},
    Responder, Scope,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{database::DatabaseTransaction, domain::DoorStateRepo, web::RestStatus};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(get_doorstate)
        .service(get_doorstate_between)
        .service(create_doorstate)
}

#[derive(Deserialize, ToSchema, Debug)]
pub struct GetDoorStateParams {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema, Debug)]
pub struct CreateDoorStateParams {
    is_open: bool,
}

#[utoipa::path(
    path = "/api/doorstate/",
    responses(
        (status = 200, description = "Success", body = DoorState),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/")]
async fn get_doorstate(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    let now = chrono::Utc::now();
    RestStatus::ok_from_result(transaction.door_state_at(now).await)
}

#[utoipa::path(
    path = "/api/doorstate/",
    responses(
        (status = 200, description = "Success", body = Vec<DoorSatae>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/between/")]
async fn get_doorstate_between(
    params: Query<GetDoorStateParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(
        transaction
            .door_state_between(params.start, params.end)
            .await,
    )
}

#[utoipa::path(
    path = "/api/doorstate/",
    responses(
        (status = 200, description = "Success"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/")]
async fn create_doorstate(
    params: web::Json<CreateDoorStateParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    let now = chrono::Utc::now();
    RestStatus::ok_from_result(transaction.create_door_state(now, params.is_open).await)
}
