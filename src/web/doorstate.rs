use actix_web::{
    get, post,
    web::{self, Query},
    Responder, Scope,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::{
    database::DatabaseTransaction,
    domain::door_state::DoorStateRepo,
    web::{auth::User, RestStatus},
};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(get_doorstate)
        .service(get_doorstate_between)
        .service(create_doorstate)
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct GetDoorStateParams {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
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
    path = "/api/doorstate/between",
    params(GetDoorStateParams),
    responses(
        (status = 200, description = "Success", body = Vec<DoorState>),
        (status = 400, description = "Bad Request"),
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
    params(CreateDoorStateParams),
    responses(
        (status = 201, description = "Created"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/")]
async fn create_doorstate(
    _user: User,
    params: web::Json<CreateDoorStateParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    let now = chrono::Utc::now();
    RestStatus::created_from_result(transaction.create_door_state(now, params.is_open).await)
}
