use actix_web::{
    delete, get, patch, post,
    web::{self, Path, Query},
    Responder, Scope,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::{
    database::{DatabaseConnection, DatabaseTransaction},
    domain::{
        legislative_period::{LegislativePeriod, LegislativePeriodRepo},
        sitzung::Sitzung,
        Result,
    },
    web::{auth, RestStatus},
};

/// Create the legislative period service
pub(crate) fn service() -> Scope {
    web::scope("/legislative")
        .service(get_legislatives)
        .service(get_legislatives_sitzungen)
        .service(create_legislative)
        .service(patch_legislative)
        .service(delete_legislative)
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct CreateLegislativeParams {
    name: String,
}

#[utoipa::path(
    path = "/api/legislative",
    responses(
        (status = 200, description = "Success", body = Vec<LegislativePeriod>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("")]
async fn get_legislatives(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.get_legislatives().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative/{id}/sitzungen",
    responses(
        (status = 200, description = "Success", body = Vec<Sitzung>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{id}/sitzungen")]
async fn get_legislatives_sitzungen(
    mut conn: DatabaseConnection,
    path_params: Path<Uuid>,
) -> Result<impl Responder> {
    let result = conn.get_legislatives_sitzungen(*path_params).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative",
    params(CreateLegislativeParams),
    responses(
        (status = 201, description = "Created", body = LegislativePeriod),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("", wrap = "auth::capability::RequireManageSitzungen")]
async fn create_legislative(
    params: Query<CreateLegislativeParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.create_legislative(params.name.clone()).await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative/{id}",
    request_body = CreateLegislativeParams,
    responses(
        (status = 200, description = "Success", body = LegislativePeriod),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{id}", wrap = "auth::capability::RequireManageSitzungen")]
async fn patch_legislative(
    path_params: Path<Uuid>,
    params: actix_web_validator::Json<CreateLegislativeParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .patch_legislative(*path_params, params.name.clone())
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative/{id}",
    responses(
        (status = 200, description = "Sccess"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{id}", wrap = "auth::capability::RequireManageSitzungen")]
async fn delete_legislative(
    path_params: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction.delete_legislative(*path_params).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(result)))
}
