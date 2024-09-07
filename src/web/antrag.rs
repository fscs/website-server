use actix_web::{
    delete, get, patch, post,
    web::{self, Path},
    Responder, Scope,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{database::DatabaseTransaction, domain::antrag::AntragRepo, web::RestStatus};

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

#[derive(Debug, IntoParams, Deserialize, ToSchema)]
pub struct CreateAntragParams {
    antragssteller: Vec<Uuid>,
    begründung: String,
    antragstext: String,
    titel: String,
}

#[derive(Debug, IntoParams, Deserialize, ToSchema)]
pub struct UpdateAntragParams {
    antragssteller: Option<Vec<Uuid>>,
    begründung: Option<String>,
    antragstext: Option<String>,
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
async fn get_anträge(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    RestStatus::ok_from_result(transaction.anträge().await)
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}",
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{antrag_id}/")]
async fn get_antrag_by_id(
    antrag_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(transaction.antrag_by_id(*antrag_id).await)
}

#[utoipa::path(
    path = "/api/anträge/",
    responses(
        (status = 201, description = "Created", body = Antrag),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/")]
async fn create_antrag(
    params: web::Json<CreateAntragParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::created_from_result(
        transaction
            .create_antrag(
                &params.antragssteller,
                &params.titel,
                &params.begründung,
                &params.antragstext,
            )
            .await,
    )
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}",
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{antrag_id}/")]
async fn patch_antrag(
    params: web::Json<UpdateAntragParams>,
    antrag_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(
        transaction
            .update_antrag(
                *antrag_id,
                params.antragssteller.as_deref(),
                params.titel.as_deref(),
                params.begründung.as_deref(),
                params.antragstext.as_deref(),
            )
            .await,
    )
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}",
    responses(
        (status = 200, description = "Success"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{antrag_id}/")]
async fn delete_antrag(
    antrag_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(transaction.delete_antrag(*antrag_id).await)
}
