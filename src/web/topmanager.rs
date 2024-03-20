
use crate::database::{DatabasePool, DatabaseTransaction};
use actix_web::{delete, get, put, web::{self, Data}, HttpResponse, Responder, Scope, HttpRequest, patch};
use actix_web::body::BoxBody;
use chrono::{DateTime, Utc};
use icalendar::ValueType::Date;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow};
use uuid::Uuid;
use crate::domain::TopManagerRepo;
use crate::domain::Antrag;

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(create_antrag)
        .service(update_antrag)
        .service(get_anträge)
        .service(delete_antrag)
        .service(get_sitzungen)
        .service(tops_by_sitzung)
        .service(anträge_by_top)
        .service(anträge_by_sitzung)
        .service(create_sitzung)
        .service(create_top)
        .service(get_current_tops_with_anträge)
}

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

#[derive(Debug, Deserialize, Clone)]
pub struct CreateTopParams {
    pub titel: String,
    pub sitzung_id: Uuid,
    pub inhalt: Option<serde_json::Value>,
    pub position: i64
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

#[derive(Debug, Serialize, FromRow)]
struct Person {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
struct TopWithAnträge {
    pub id: Uuid,
    pub position: i64,
    pub name: String,
    pub anträge: Vec<Antrag>,
    pub inhalt: Option<serde_json::Value>
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
            Ok((transaction.create_antrag(&params.titel, &params.antragstext, &params.begründung).await?, transaction))
        }
    }).await;

    RestStatus::created_from_result(result)
}

async fn save_antrag(params: &CreateAntragParams, now: DateTime<Utc>, transaction: &mut DatabaseTransaction<'_>) -> anyhow::Result<Antrag> {
    let antrag = transaction.create_antrag(&params.titel, &params.antragstext, &params.begründung).await?;

    //check if there is a sitzung in the future
    let Some(sitzung) = transaction.find_sitzung_after(now.naive_utc()).await? else {
        return Ok(antrag);
    };

    //create new top
    let top = transaction.create_top(&antrag.titel, sitzung.id, None).await?;

    //create mapping between top and antrag
    transaction.add_antrag_to_top(antrag.id, top.id).await?;

    Ok(antrag)
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
        }}).await;

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

#[get("/sitzung")]
async fn get_sitzungen(db: Data<DatabasePool>) -> impl Responder {
    let result = db.transaction(move |mut transaction| {
        async move {
            Ok((transaction.get_sitzungen().await?, transaction))
        }
    }).await;

    RestStatus::ok_from_result(result)
}

#[put("/sitzung")]
async fn create_sitzung(
    db: Data<DatabasePool>,
    params: web::Json<CreateSitzungParams>,
) -> impl Responder {
    let result = db.transaction(move |mut transaction| {
        let params = params.clone();
        async move {
            Ok((transaction.create_sitzung(params.datum, params.name.as_str()).await?, transaction))
        }
    }).await;

    RestStatus::created_from_result(result)

}
#[put("/sitzung/{sitzung_id}/top")]
async fn create_top(db: Data<DatabasePool>, sitzung_id: web::Path<Uuid>, params: web::Json<CreateTopParams>) -> impl Responder {

    let result = db.transaction(move |mut transaction| {
        let params = params.clone();
        let sitzung_id = sitzung_id.clone();
        async move {
            Ok((transaction.create_top(&params.titel, sitzung_id, params.inhalt).await?, transaction))
        }
    }).await;

    RestStatus::created_from_result(result)
}

#[get("/sitzung/{id}/tops")]
async fn tops_by_sitzung(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let tops = db.transaction(move |mut transaction| {
        let id = id.clone();
        async move {
            Ok((transaction.tops_by_sitzung(id.clone()).await?, transaction))
        }
    }).await;

    RestStatus::ok_from_result(tops)
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

#[get("/current_tops")]
async fn get_current_tops_with_anträge(db: Data<DatabasePool>) -> impl Responder {

    let tops_with_anträge : Option<anyhow::Result<Vec<TopWithAnträge>>> = db.transaction(move |mut transaction| async move {
        let now = chrono::Utc::now();
        let Some(next_sitzung) = transaction.find_sitzung_after(now.naive_utc()).await? else {
            return Ok((None, transaction));
        };

        let tops = transaction.tops_by_sitzung(next_sitzung.id).await?;

        let mut tops_with_anträge = vec![];

        for top in tops {
            let anträge = transaction.anträge_by_top(top.id).await?;
            let top_with_anträge = TopWithAnträge {
                id: top.id,
                position: top.position,
                name: top.name,
                anträge,
                inhalt: top.inhalt,
            };
            tops_with_anträge.push(top_with_anträge);
        }

        Ok((Some(tops_with_anträge), transaction))
    }).await.transpose();

    match tops_with_anträge {
        Some(tops_with_anträge) => RestStatus::ok_from_result(tops_with_anträge),
        None => RestStatus::NotFound
    }
}

#[get("/sitzung/{id}/anträge")]
async fn anträge_by_sitzung(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let anträge = db.transaction(move |mut transaction| {
        let id = id.clone();
        async move {
            Ok((transaction.anträge_by_sitzung(id.clone()).await?, transaction))
        }
    }).await;

    RestStatus::ok_from_result(anträge)
}
