use std::borrow::Cow;

use actix_web::{
    get, post,
    web::{self, Json as ActixJson},
    Responder, Scope,
};
use actix_web_validator::Query;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::{Validate, ValidationError};

use crate::{
    database::{DatabaseConnection, DatabaseTransaction},
    domain::{door_state::{DoorState, DoorStateRepo}, Result},
    web::{auth::User, RestStatus},
};

/// Create the doorstate service under /doorstate
pub(crate) fn service() -> Scope {
    web::scope("/doorstate")
        .service(get_doorstate)
        .service(get_doorstate_between)
        .service(create_doorstate)
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
#[validate(schema(function = "validate_doorstate_params"))]
pub struct GetDoorStateParams {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

fn validate_doorstate_params(
    params: &GetDoorStateParams,
) -> core::result::Result<(), ValidationError> {
    if params.start > params.end {
        Err(ValidationError::new("doorstate_params")
            .with_message(Cow::Borrowed("start must be before end")))
    } else {
        Ok(())
    }
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
async fn get_doorstate(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let now = chrono::Utc::now();
    let result = conn.door_state_at(now).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/doorstate/between/",
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
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.door_state_between(params.start, params.end).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/doorstate/",
    params(CreateDoorStateParams),
    request_body = CreateDoorStateParams,
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
    params: ActixJson<CreateDoorStateParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let now = chrono::Utc::now();
    let result = transaction.create_door_state(now, params.is_open).await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(result)))
}
