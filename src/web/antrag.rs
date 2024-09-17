use actix_web::{
    delete, get, patch, post,
    web::{self, Path},
    Responder, Scope,
};
use actix_web_validator::Json as ActixJson;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::{
    database::{DatabaseConnection, DatabaseTransaction},
    domain::{antrag::AntragRepo, Result},
    web::{auth::User, RestStatus},
};

pub(crate) fn service(path: &'static str) -> Scope {
    let scope = web::scope(path).service(get_anträge).service(create_antrag);

    // must come last
    register_antrag_id_service(scope)
}

fn register_antrag_id_service(parent: Scope) -> Scope {
    parent
        .service(get_antrag_by_id)
        .service(patch_antrag)
        .service(delete_antrag)
}

#[derive(Debug, IntoParams, Deserialize, ToSchema, Validate)]
pub struct CreateAntragParams {
    antragssteller: Vec<Uuid>,
    #[validate(length(min = 1))]
    begründung: String,
    #[validate(length(min = 1))]
    antragstext: String,
    #[validate(length(min = 1))]
    titel: String,
}

#[derive(Debug, IntoParams, Deserialize, ToSchema, Validate)]
pub struct UpdateAntragParams {
    antragssteller: Option<Vec<Uuid>>,
    #[validate(length(min = 1))]
    begründung: Option<String>,
    #[validate(length(min = 1))]
    antragstext: Option<String>,
    #[validate(length(min = 1))]
    titel: Option<String>,
}

#[utoipa::path(
    path = "/api/anträge/",
    responses(
        (status = 200, description = "Success", body = Vec<Antrag>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/")]
async fn get_anträge(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.anträge().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}/",
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{antrag_id}/")]
async fn get_antrag_by_id(
    antrag_id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.antrag_by_id(*antrag_id).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/anträge/",
    request_body = CreateAntragParams,
    responses(
        (status = 201, description = "Created", body = Antrag),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/")]
async fn create_antrag(
    _user: User,
    params: ActixJson<CreateAntragParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .create_antrag(
            &params.antragssteller,
            &params.titel,
            &params.begründung,
            &params.antragstext,
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}/",
    request_body = UpdateAntragParams,
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{antrag_id}/")]
async fn patch_antrag(
    _user: User,
    params: ActixJson<UpdateAntragParams>,
    antrag_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .update_antrag(
            *antrag_id,
            params.antragssteller.as_deref(),
            params.titel.as_deref(),
            params.begründung.as_deref(),
            params.antragstext.as_deref(),
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}/",
    responses(
        (status = 200, description = "Success"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{antrag_id}/")]
async fn delete_antrag(
    _user: User,
    antrag_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction.delete_antrag(*antrag_id).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}
