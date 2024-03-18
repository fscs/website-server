use crate::database::{DatabasePool, DatabaseTransaction};
use actix_web::{delete, get, put, web::{self, Data}, HttpResponse, Responder, Scope, HttpRequest};
use actix_web::body::BoxBody;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Row};
use uuid::Uuid;
use crate::domain::SitzungRepo;
use crate::domain::Antrag;

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(create_antrag)
        .service(get_anträge)
        .service(delete_antrag)
        .service(get_sitzungen)
        .service(tops_by_sitzung)
        .service(anträge_by_top)
        .service(anträge_by_sitzung)
        .service(create_sitzung)
}

#[derive(Debug, Deserialize, Clone)]
pub struct CreateAntragParams {
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteAntragParams {
    pub id: Uuid,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Top {
    pub id: Uuid,
    pub position: i32,
    pub sitzung_id: Uuid,
    pub name: String,
    pub inhalt: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTopParams {
    pub titel: String,
    pub sitzung_id: Uuid,
    pub inhalt: Option<serde_json::Value>,
    pub position: i32,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Sitzung {
    pub id: Uuid,
    pub datum: chrono::NaiveDateTime,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
struct CreateSitzungParams {
    pub datum: chrono::NaiveDateTime,
    pub name: String,
}

enum RestStatus {
    Ok(serde_json::Value),
    Created(serde_json::Value),
    NotFound,
    Error(anyhow::Error)
}

impl RestStatus {
    fn created_from_result<T: Serialize>(result: anyhow::Result<T>) -> RestStatus {
        match result {
            Ok(antrag) => match serde_json::to_value(antrag) {
                Ok(value) => RestStatus::Created(value),
                Err(e) => RestStatus::Error(anyhow::Error::from(e))
            },
            Err(e) => RestStatus::Error(anyhow::Error::from(e))
        }
    }

    fn ok_from_result<T: Serialize>(result: anyhow::Result<T>) -> RestStatus {
        match result {
            Ok(antrag) => match serde_json::to_value(antrag) {
                Ok(value) => RestStatus::Ok(value),
                Err(e) => RestStatus::Error(anyhow::Error::from(e))
            },
            Err(e) => RestStatus::Error(anyhow::Error::from(e))
        }
    }
}

impl Responder for RestStatus {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            RestStatus::Ok(value) => {
                HttpResponse::Ok()
                    .json(value)
            }
            RestStatus::Created(value) => {
                log::debug!("Created: {:?}", value.as_str());
                HttpResponse::Created()
                    .json(value)
            }
            RestStatus::NotFound => {
                log::debug!("Resource {} not found", req.path());
                HttpResponse::NotFound().body("Not Found")
            }
            RestStatus::Error(error) => {
                log::error!("{:?}", error);
                HttpResponse::InternalServerError().body("Internal Server Error")
            }
        }
    }
}

#[put("/antrag")]
async fn create_antrag(
    db: Data<DatabasePool>,
    params: web::Json<CreateAntragParams>,
) -> impl Responder {
    let now = chrono::Utc::now();
    let params = params.0;

    let antrag = db.transaction(move |mut transaction| {
        let params = params.clone();
        async move {
        Ok((save_antrag(&params.clone(), now, &mut transaction).await?, transaction))
    }}).await;

    RestStatus::created_from_result(antrag)
}

async fn save_antrag(params: &CreateAntragParams, now: DateTime<Utc>, transaction: &mut DatabaseTransaction<'_>) -> anyhow::Result<Antrag> {
    let antrag = sqlx::query_as::<_, Antrag>(
        "INSERT INTO anträge (titel, antragstext, begründung) VALUES ($1, $2, $3) RETURNING *",
    )
        .bind(&params.titel)
        .bind(&params.antragstext)
        .bind(&params.begründung)
        .fetch_one(&mut **transaction)
        .await?;

    //check if there is a sitzung in the future
    let sitzung = transaction.find_sitzung_after(now.naive_utc()).await?;

    let Some(sitzung) = sitzung else {
        return Ok(antrag);
    };

    //get the last created top
    let top = sqlx::query_as::<_, Top>("SELECT * FROM tops WHERE sitzung_id = $1 ORDER BY id DESC LIMIT 1")
        .bind(sitzung.id)
        .fetch_optional(&mut **transaction)
        .await?;

    //create new top
    let top = sqlx::query_as::<_, Top>(
        "INSERT INTO tops (name, sitzung_id, position) VALUES ($1, $2, $3) RETURNING *",
    )
        .bind(&params.titel)
        .bind(sitzung.id)
        .bind(top.map_or(0, |x| x.position + 1))
        .fetch_one(&mut **transaction)
        .await?;

    //create mapping between top and antrag
    sqlx::query("INSERT INTO antragstop (antrag_id, top_id) VALUES ($1, $2)")
        .bind(antrag.id)
        .bind(top.id)
        .execute(&mut **transaction)
        .await?;

    Ok(antrag)
}

#[get("/get_anträge")]
async fn get_anträge(db: Data<DatabasePool>) -> impl Responder {
    match sqlx::query_as::<_, Antrag>("SELECT * FROM anträge")
        .fetch_all(db.pool())
        .await
    {
        Ok(anträge) => HttpResponse::Ok().json(anträge),
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get Anträge: {:?}", e)),
    }
}

#[delete("/antrag")]
async fn delete_antrag(
    db: Data<DatabasePool>,
    params: web::Query<DeleteAntragParams>,
) -> impl Responder {
    let result = sqlx::query("DELETE FROM anträge WHERE id = $1")
        .bind(params.id)
        .execute(db.pool());
    match result.await {
        Ok(_) => "Antrag gelöscht",
        Err(e) => {
            log::error!("Failed to delete Antrag: {:?}", e);
            "Failed to delete Antrag"
        }
    }
}

#[get("/sitzungen")]
async fn get_sitzungen(db: Data<DatabasePool>) -> impl Responder {
    match sqlx::query_as::<_, Sitzung>("SELECT * FROM sitzungen")
        .fetch_all(db.pool())
        .await
    {
        Ok(sitzungen) => HttpResponse::Ok().json(sitzungen),
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get Sitzungen: {:?}", e)),
    }
}

#[put("/sitzung")]
async fn create_sitzung(
    db: Data<DatabasePool>,
    params: web::Json<CreateSitzungParams>,
) -> impl Responder {
    let result = db.transaction(move |mut transaction| {
        let params = params.0.clone();
        async move {
            let params = params.clone();
            Ok((transaction.create_sitzung(params.datum, &params.name).await?, transaction))
    }}).await;

    RestStatus::ok_from_result({|| Ok(result?)}())
}

#[put("/top")]
async fn create_top(db: Data<DatabasePool>, params: web::Json<CreateTopParams>) -> impl Responder {

    let result = sqlx::query("INSERT INTO tops (name, sitzung_id, inhalt) VALUES ($1, $2, $3")
        .bind(&params.titel)
        .bind(params.sitzung_id)
        .bind(&params.inhalt)
        .execute(db.pool());
    match result.await {
        Ok(_) => "Top erstellt",
        Err(e) => {
            log::error!("Failed to create Top: {:?}", e);
            "Failed to create Top"
        }
    }
}

#[get("/sitzungen/{id}/tops")]
async fn tops_by_sitzung(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let topids = sqlx::query("SELECT top_id FROM sitzungstop WHERE sitzung_id = $1")
        .bind(*id)
        .fetch_all(db.pool())
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| row.get::<Uuid, _>("top_id"))
                .collect::<Vec<_>>()
        });

    match topids {
        Ok(topids) => {
            let tops = sqlx::query_as::<_, Top>("SELECT * FROM tops WHERE id = ANY($1)")
                .bind(&topids)
                .fetch_all(db.pool())
                .await;
            match tops {
                Ok(tops) => HttpResponse::Ok().json(tops),
                Err(e) => HttpResponse::NotFound().json(format!("Failed to get Tops: {:?}", e)),
            }
        }
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get TopIds: {:?}", e)),
    }
}

#[get("/tops/{topid}/anträge")]
async fn anträge_by_top(db: Data<DatabasePool>, topid: web::Path<Uuid>) -> impl Responder {
    let anträge = sqlx::query_as::<_, Antrag>(
        "SELECT * From anträge Join antragstop ON anträge.id = antragstop.antrag_id WHERE top_id = $1",
    )
    .bind(*topid)
    .fetch_all(db.pool())
    .await;
    match anträge {
        Ok(anträge) => HttpResponse::Ok().json(anträge),
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get Anträge: {:?}", e)),
    }
}

#[get("/sitzungen/{id}/anträge")]
async fn anträge_by_sitzung(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let anträge = sqlx::query_as::<_, Antrag>(
        "SELECT * FROM anträge
        JOIN antragstop ON anträge.id = antragstop.antrag_id
        JOIN public.sitzungen s2 on tops.sitzung_id = s2.id
        WHERE s2.id = $1",
    )
    .bind(*id)
    .fetch_all(db.pool())
    .await;

    match anträge {
        Ok(anträge) => HttpResponse::Ok().json(anträge),
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get Anträge: {:?}", e)),
    }
}
