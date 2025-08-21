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
        legislative_periods::{LegislativePeriod, LegislativePeriodRepo},
        sitzung::Sitzung,
        Result,
    },
    web::{auth, RestStatus},
};

/// Create the legislative period service
pub(crate) fn service() -> Scope {
    let scope = web::scope("/legislative-periods")
        .service(get_legislatives)
        .service(create_legislative_period);

    // must come last
    register_legislative_period_id_service(scope)
}

fn register_legislative_period_id_service(parent: Scope) -> Scope {
    parent
        .service(get_legislative_period_sitzungen)
        .service(get_legislative_by_id)
        .service(patch_legislative_period)
        .service(delete_legislative_period)
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct CreateLegislativeParams {
    name: String,
}

#[utoipa::path(
    path = "/api/legislative-periods",
    responses(
        (status = 200, description = "Success", body = Vec<LegislativePeriod>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("")]
async fn get_legislatives(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.legislative_periods().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative-periods/{id}",
    responses(
        (status = 200, description = "Success", body = LegislativePeriod),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{id}")]
async fn get_legislative_by_id(
    id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.legislativ_period_by_id(*id).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/legislative-periods/{id}/sitzungen",
    responses(
        (status = 200, description = "Success", body = Vec<Sitzung>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{id}/sitzungen")]
async fn get_legislative_period_sitzungen(
    mut conn: DatabaseConnection,
    id: Path<Uuid>,
) -> Result<impl Responder> {
    let result = conn.legislative_period_sitzungen(*id).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative-periods",
    params(CreateLegislativeParams),
    responses(
        (status = 201, description = "Created", body = LegislativePeriod),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("", wrap = "auth::capability::RequireManageSitzungen")]
async fn create_legislative_period(
    params: Query<CreateLegislativeParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.create_legislative_period(params.name.clone()).await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative-periods/{id}",
    request_body = CreateLegislativeParams,
    responses(
        (status = 200, description = "Success", body = LegislativePeriod),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{id}", wrap = "auth::capability::RequireManageSitzungen")]
async fn patch_legislative_period(
    id: Path<Uuid>,
    params: actix_web_validator::Json<CreateLegislativeParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .update_legislative_period(*id, params.name.clone())
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative-periods/{id}",
    responses(
        (status = 200, description = "Sccess"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{id}", wrap = "auth::capability::RequireManageSitzungen")]
async fn delete_legislative_period(
    id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction.delete_legislative_period(*id).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(result)))
}
