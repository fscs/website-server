#[derive(Debug, Deserialize, Clone)]
pub struct CreateAntragParams {
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
    pub antragssteller: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAntragParams {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteAntragParams {
    pub id: Uuid,
}

use actix_web::{delete, get, patch, put, Responder, web};
use actix_web::web::Data;
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;
use crate::database::{DatabasePool};
use crate::domain::{TopManagerRepo};
use crate::web::topmanager::RestStatus;

#[patch("/antrag")]
async fn update_antrag(
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

#[put("/antrag")]
async fn create_antrag(
    db: Data<DatabasePool>,
    params: web::Json<CreateAntragParams>,
) -> impl Responder {
    let result = db.transaction(move |mut transaction| {
        let params = params.clone();
        async move {
            let antrag = transaction.create_antrag(&params.titel, &params.antragstext, &params.begründung).await?;
            let now = Utc::now();

            let Some(sitzung) = transaction.find_sitzung_after(now.naive_utc()).await? else {
                return Ok((antrag, transaction));
            };

            let top = transaction.create_top(&antrag.titel, sitzung.id, None).await?;
            transaction.add_antrag_to_top(antrag.id, top.id).await?;

            Ok((antrag, transaction))
        }
    }).await;

    RestStatus::created_from_result(result)
}

#[get("/antrag")]
async fn get_anträge(db: Data<DatabasePool>) -> impl Responder {
    let anträge = db.transaction(move |mut transaction| {
        async move {
            Ok((transaction.get_anträge().await?, transaction))
        }
    }).await;

    RestStatus::ok_from_result(anträge)
}

#[get("/antrag/{id}")]
async fn get_antrag(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let antrag = db.transaction(move |mut transaction| {
        let id = id.clone();
        async move {
            Ok((transaction.find_antrag_by_id(id.clone()).await?, transaction))
        }
    }).await;

    RestStatus::ok_from_result(antrag)
}

#[delete("/antrag/{id}")]
async fn delete_antrag(
    db: Data<DatabasePool>,
    id: web::Path<Uuid>,
) -> impl Responder {
    let result = db.transaction(move |mut transaction| {
        let id = id.clone();
        async move {
            Ok((transaction.delete_antrag(id).await?, transaction))
        }
    }).await;

    RestStatus::ok_from_result(result)
}