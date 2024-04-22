#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct CreateAntragParams {
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
    pub antragssteller: String,
}

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct UpdateAntragParams {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct DeleteAntragParams {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct CreateAntragTopMappingParams {
    pub antrag_id: Uuid,
    pub top_id: Uuid,
}

use crate::database::DatabasePool;
use crate::domain::{TopManagerRepo};
use crate::web::auth::User;
use crate::web::topmanager::RestStatus;
use actix_web::web::{Data};
use actix_web::{delete, get, patch, put, web, Responder};
use chrono::Utc;
use serde::Deserialize;


use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[utoipa::path(
    path = "/api/topmanager/antrag/",
    request_body=UpdateAntragParams,
    responses(
        (status = 201, description = "Created", body = Antrag),
        (status = 400, description = "Bad Request"),
    )
)]
#[patch("/antrag/")]
async fn update_antrag(
    _user: User,
    db: Data<DatabasePool>,
    params: web::Json<UpdateAntragParams>,
) -> impl Responder {
    let result = sqlx::query(
        "UPDATE anträge SET titel = $1, antragstext = $2, begründung = $3 WHERE id = $4",
    )
    .bind(&params.titel)
    .bind(&params.antragstext)
    .bind(&params.begründung)
    .bind(params.id)
    .execute(db.pool())
    .await;
    match result {
        Ok(_) => "Antrag geändert",
        Err(e) => {
            log::error!("Failed to update Antrag: {:?}", e);
            "Failed to update Antrag"
        }
    }
}

#[utoipa::path(
    path = "/api/topmanager/antrag/",
    request_body = CreateAntragParams,
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/antrag/")]
async fn create_antrag(
    _user: User,
    db: Data<DatabasePool>,
    params: web::Json<CreateAntragParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let antrag = transaction
                    .create_antrag(&params.titel, &params.antragstext, &params.begründung)
                    .await?;

                let person = transaction.create_person(&params.antragssteller).await?;
                transaction
                    .create_antragssteller(antrag.id, person.id)
                    .await?;

                let now = Utc::now();

                let Some(sitzung) = transaction.find_sitzung_after(now.naive_utc()).await? else {
                    return Ok((antrag, transaction));
                };

                let top = transaction
                    .create_top(&antrag.titel, sitzung.id, None)
                    .await?;
                transaction.add_antrag_to_top(antrag.id, top.id).await?;

                Ok((antrag, transaction))
            }
        })
        .await;

    RestStatus::created_from_result(result)
}

#[utoipa::path(
    path = "/api/topmanager/top/{top_id}/antrag/",
    params(("top_id" = Uuid, Path,)),
    request_body = CreateAntragParams,
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/top/{top_id}/antrag/")]
async fn create_antrag_for_top(
    _user: User,
    db: Data<DatabasePool>,
    top_id: web::Path<Uuid>,
    params: web::Json<CreateAntragParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            let top_id = top_id.clone();
            async move {
                let antrag = transaction
                    .create_antrag(&params.titel, &params.antragstext, &params.begründung)
                    .await?;

                let person = transaction.create_person(&params.antragssteller).await?;
                transaction
                    .create_antragssteller(antrag.id, person.id)
                    .await?;

                transaction.add_antrag_to_top(antrag.id, top_id).await?;

                Ok((antrag, transaction))
            }
        })
        .await;

    RestStatus::created_from_result(result)
}

#[utoipa::path(
    path = "/api/topmanager/antrag/",
    responses(
        (status = 200, description = "Success", body = Vec<Antrag>),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/antrag/")]
async fn get_anträge(db: Data<DatabasePool>) -> impl Responder {
    let anträge = db
        .transaction(move |mut transaction| async move {
            Ok((transaction.get_anträge().await?, transaction))
        })
        .await;

    RestStatus::ok_from_result(anträge)
}

#[utoipa::path(
    get,
    path = "/api/topmanager/antrag/{id}/",
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/antrag/{id}/")]
async fn get_antrag(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let antrag = db
        .transaction(move |mut transaction| {
            let id = id.clone();
            async move { Ok((transaction.find_antrag_by_id(id).await?, transaction)) }
        })
        .await;

    RestStatus::ok_from_result(antrag)
}

#[utoipa::path(
    path = "/api/topmanager/antrag/{id}/",
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 400, description = "Bad Request"),
    )
)]
#[delete("/antrag/{id}/")]
async fn delete_antrag(_user: User, db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let id = id.clone();
            async move { Ok((transaction.delete_antrag(id).await?, transaction)) }
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[utoipa::path(
    path = "/api/topmanager/antrag/assoc/",
    responses(
        (status = 200, description = "Success", body = AntragTopMapping),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/antrag/assoc/")]
async fn put_antrag_top_mapping(
    _user: User,
    db: Data<DatabasePool>,
    params: web::Json<CreateAntragTopMappingParams>,
) -> impl Responder {
    let antrag = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                Ok((
                    transaction
                        .create_antrag_top_mapping(params.antrag_id, params.top_id)
                        .await?,
                    transaction,
                ))
            }
        })
        .await;

    RestStatus::ok_from_result(antrag)
}

#[utoipa::path(
    path = "/api/topmanager/antrag/assoc/",
    responses(
        (status = 200, description = "Success", body = AntragTopMapping),
        (status = 400, description = "Bad Request"),
    )
)]
#[delete("/antrag/assoc/")]
async fn delete_antrag_top_mapping(
    _user: User,
    db: Data<DatabasePool>,
    params: web::Json<CreateAntragTopMappingParams>,
) -> impl Responder {
    let antrag = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                Ok((
                    transaction
                        .delete_antrag_top_mapping(params.antrag_id, params.top_id)
                        .await?,
                    transaction,
                ))
            }
        })
        .await;

    RestStatus::ok_from_result(antrag)
}
