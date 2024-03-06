use crate::database::DatabasePool;
use actix_web::{
    get, post,
    web::{self, Data},
    HttpResponse, Responder, Scope,
};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path).service(create_antrag).service(get_anträge)
}

#[derive(Debug, Deserialize)]
pub struct CreateAntragParams {
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Antrag {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[post("/create_Antrag")]
async fn create_antrag(
    db: Data<DatabasePool>,
    params: web::Query<CreateAntragParams>,
) -> impl Responder {
    let result =
        sqlx::query("INSERT INTO anträge (titel, antragstext, begründung) VALUES ($1, $2, $3)")
            .bind(&params.titel)
            .bind(&params.antragstext)
            .bind(&params.begründung)
            .execute(db.pool());

    match result.await {
        Ok(_) => "Antrag erstellt",
        Err(e) => {
            log::error!("Failed to create Antrag: {:?}", e);
            "Failed to create Antrag"
        }
    }
}

#[get("/get_Anträge")]
async fn get_anträge(db: Data<DatabasePool>) -> impl Responder {
    match sqlx::query_as::<_, Antrag>("SELECT * FROM anträge")
        .fetch_all(db.pool())
        .await
    {
        Ok(anträge) => HttpResponse::Ok().json(anträge),
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get Anträge: {:?}", e)),
    }
}
