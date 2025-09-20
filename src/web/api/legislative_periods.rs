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
        legislatur_periode::{LegislaturPeriode, LegislaturPeriodeRepo},
        sitzung::Sitzung,
        Result,
    },
    web::{auth, RestStatus},
};

/// Create the legislative period service
pub(crate) fn service() -> Scope {
    let scope = web::scope("/legislative-periods")
        .service(get_legislatur_perioden)
        .service(create_legislatur_periode);

    // must come last
    register_legislative_period_id_service(scope)
}

fn register_legislative_period_id_service(parent: Scope) -> Scope {
    parent
        .service(get_sitzungen_by_legislatur_periode)
        .service(get_legislatur_periode_by_id)
        .service(patch_legislatur_periode)
        .service(delete_legislatur_periode)
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct CreateLegislativeParams {
    name: String,
}

#[utoipa::path(
    path = "/api/legislative-periods",
    responses(
        (status = 200, description = "Success", body = Vec<LegislaturPeriode>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("")]
async fn get_legislatur_perioden(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.legislatur_perioden().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative-periods/{id}",
    responses(
        (status = 200, description = "Success", body = LegislaturPeriode),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{id}")]
async fn get_legislatur_periode_by_id(
    id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.legislatur_periode_by_id(*id).await?;

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
async fn get_sitzungen_by_legislatur_periode(
    mut conn: DatabaseConnection,
    id: Path<Uuid>,
) -> Result<impl Responder> {
    let result = conn.sitzungen_by_legislatur_periode(*id).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative-periods",
    params(CreateLegislativeParams),
    responses(
        (status = 201, description = "Created", body = LegislaturPeriode),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("", wrap = "auth::capability::RequireManageSitzungen")]
async fn create_legislatur_periode(
    params: Query<CreateLegislativeParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.create_legislatur_periode(params.name.clone()).await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/legislative-periods/{id}",
    request_body = CreateLegislativeParams,
    responses(
        (status = 200, description = "Success", body = LegislaturPeriode),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{id}", wrap = "auth::capability::RequireManageSitzungen")]
async fn patch_legislatur_periode(
    id: Path<Uuid>,
    params: actix_web_validator::Json<CreateLegislativeParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .update_legislatur_periode(*id, params.name.clone())
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
async fn delete_legislatur_periode(
    id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction.delete_legislatur_periode(*id).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(result)))
}
