use std::borrow::Cow;

use actix_web::web::Path;
use actix_web::{delete, get, patch, post, web, Responder, Scope};
use actix_web_validator::{Json as ActixJson, Query};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::database::{DatabaseConnection, DatabaseTransaction};

use crate::domain::{
    self,
    antrag_top_map::AntragTopMapRepo,
    sitzung::{SitzungKind, SitzungRepo, TopKind},
    Result,
};
use crate::web::{auth::User, RestStatus};

pub(crate) fn service(path: &'static str) -> Scope {
    let scope = web::scope(path)
        .service(get_sitzungen)
        .service(post_sitzungen)
        .service(get_sitzungen_between)
        .service(get_first_sitzung_after);

    // must come last
    register_sitzung_id_service(scope)
}

fn register_sitzung_id_service(parent: Scope) -> Scope {
    let scope = parent
        .service(get_sitzung_by_id)
        .service(patch_sitzung_by_id)
        .service(delete_sitzung_by_id)
        .service(get_abmeldungen_by_sitzung)
        .service(post_tops);

    // must come last
    register_top_id_service(scope)
}

fn register_top_id_service(parent: Scope) -> Scope {
    parent
        .service(patch_tops)
        .service(delete_tops)
        .service(assoc_antrag)
        .service(delete_assoc_antrag)
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct CreateSitzungParams {
    datetime: DateTime<Utc>,
    #[validate(length(min = 1))]
    location: String,
    kind: SitzungKind,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct CreateTopParams {
    #[validate(length(min = 1))]
    name: String,
    kind: TopKind,
    inhalt: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct UpdateSitzungParams {
    datetime: Option<DateTime<Utc>>,
    #[validate(length(min = 1))]
    location: Option<String>,
    kind: Option<SitzungKind>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct UpdateTopParams {
    #[validate(length(min = 1))]
    name: Option<String>,
    kind: Option<TopKind>,
    inhalt: Option<serde_json::Value>,
    weight: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct FirstSitzungAfterParams {
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
#[validate(schema(function = "validate_sitzung_between_params"))]
pub struct SitzungBetweenParams {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

fn validate_sitzung_between_params(
    params: &SitzungBetweenParams,
) -> core::result::Result<(), ValidationError> {
    if params.start > params.end {
        Err(ValidationError::new("sitzung_between_params")
            .with_message(Cow::Borrowed("start must be before end")))
    } else {
        Ok(())
    }
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct AssocAntragParams {
    antrag_id: Uuid,
}

#[utoipa::path(
    path = "/api/sitzungen/",
    responses(
        (status = 200, description = "Success", body = Vec<Sitzung>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/")]
async fn get_sitzungen(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.sitzungen().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/",
    request_body = CreateSitzungParams,
    responses(
        (status = 201, description = "Created", body = Sitzung),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/")]
async fn post_sitzungen(
    _user: User,
    params: ActixJson<CreateSitzungParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .create_sitzung(params.datetime, params.location.as_str(), params.kind)
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/first-after/",
    params(FirstSitzungAfterParams),
    responses(
        (status = 200, description = "Success", body = SitzungWithTops),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/first-after/")]
async fn get_first_sitzung_after(
    params: Query<FirstSitzungAfterParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = domain::sitzung_after_with_tops(&mut *conn, params.timestamp).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/between/",
    params(SitzungBetweenParams),
    responses(
        (status = 200, description = "Success", body = Vec<Sitzung>),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/between/")]
async fn get_sitzungen_between(
    params: Query<SitzungBetweenParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.sitzungen_between(params.start, params.end).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/",
    responses(
        (status = 200, description = "Success", body = SitzungWithTops),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{sitzung_id}/")]
async fn get_sitzung_by_id(
    sitzung_id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = domain::sitzung_with_tops(&mut *conn, *sitzung_id).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/",
    request_body = UpdateSitzungParams,
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{sitzung_id}/")]
async fn patch_sitzung_by_id(
    _user: User,
    sitzung_id: Path<Uuid>,
    params: ActixJson<UpdateSitzungParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .update_sitzung(
            *sitzung_id,
            params.datetime,
            params.location.as_deref(),
            params.kind,
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/",
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{sitzung_id}/")]
async fn delete_sitzung_by_id(
    _user: User,
    sitzung_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction.delete_sitzung(*sitzung_id).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/abmeldungen/",
    responses(
        (status = 200, description = "Success", body = Vec<Abmeldung>),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{sitzung_id}/abmeldungen/")]
async fn get_abmeldungen_by_sitzung(
    sitzung_id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    if conn.sitzung_by_id(*sitzung_id).await?.is_none() {
        return Ok(RestStatus::Success(None));
    }

    let result = domain::abmeldungen_by_sitzung(&mut *conn, *sitzung_id).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops/",
    request_body = CreateTopParams,
    responses(
        (status = 201, description = "Created", body = Top),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/{sitzung_id}/tops/")]
async fn post_tops(
    _user: User,
    sitzung_id: Path<Uuid>,
    params: ActixJson<CreateTopParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    if transaction.sitzung_by_id(*sitzung_id).await?.is_none() {
        return Ok(RestStatus::Success(None));
    }

    let result = transaction
        .create_top(
            *sitzung_id,
            params.name.as_str(),
            params.inhalt.as_ref(),
            params.kind,
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops/{top_id}/",
    request_body = UpdateTopParams,
    responses(
        (status = 200, description = "Sucess", body = Top),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{sitzung_id}/tops/{top_id}/")]
async fn patch_tops(
    _user: User,
    path_params: Path<(Uuid, Uuid)>,
    params: ActixJson<UpdateTopParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (sitzung_id, top_id) = path_params.into_inner();
    
    if transaction.sitzung_by_id(sitzung_id).await?.is_none() {
        return Ok(RestStatus::Success(None));
    }

    let result = transaction
        .update_top(
            top_id,
            None, // we dont allow moving tops
            params.name.as_deref(),
            params.inhalt.as_ref(),
            params.kind,
            params.weight,
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops/{top_id}/",
    responses(
        (status = 200, description = "Sucess", body = Top),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{sitzung_id}/tops/{top_id}/")]
async fn delete_tops(
    _user: User,
    path_params: Path<(Uuid, Uuid)>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (sitzung_id, top_id) = path_params.into_inner();

    if transaction.sitzung_by_id(sitzung_id).await?.is_none() {
        return Ok(RestStatus::Success(None));
    }

    let result = transaction.delete_top(top_id).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops/{top_id}/assoc/",
    request_body = AssocAntragParams,
    responses(
        (status = 200, description = "Sucess", body = AntragTopMapping),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{sitzung_id}/tops/{top_id}/assoc/")]
async fn assoc_antrag(
    _user: User,
    path_params: Path<(Uuid, Uuid)>,
    params: ActixJson<AssocAntragParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (sitzung_id, top_id) = path_params.into_inner();

    if transaction.sitzung_by_id(sitzung_id).await?.is_none() {
        return Ok(RestStatus::Success(None));
    }

    if transaction.top_by_id(top_id).await?.is_none() {
        return Ok(RestStatus::Success(None));
    }

    let result = transaction
        .attach_antrag_to_top(params.antrag_id, top_id)
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops/{top_id}/assoc/",
    request_body = AssocAntragParams,
    responses(
        (status = 200, description = "Sucess", body = AntragTopMapping),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{sitzung_id}/tops/{top_id}/assoc/")]
async fn delete_assoc_antrag(
    _user: User,
    path_params: Path<(Uuid, Uuid)>,
    params: ActixJson<AssocAntragParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (sitzung_id, top_id) = path_params.into_inner();

    if transaction.sitzung_by_id(sitzung_id).await?.is_none() {
        return Ok(RestStatus::Success(None));
    }

    if transaction.top_by_id(top_id).await?.is_none() {
        return Ok(RestStatus::Success(None));
    }

    let result = transaction
        .detach_antrag_from_top(params.antrag_id, top_id)
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}
