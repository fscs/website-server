use crate::database::DatabasePool;
use crate::domain::Antrag;
use crate::domain::TopManagerRepo;
use crate::web::topmanager::antrag::{create_antrag, delete_antrag, get_anträge, update_antrag};
use crate::web::topmanager::sitzungen::{
    create_sitzung, create_top, get_next_sitzung, get_sitzungen, tops_by_sitzung,
};
use crate::web::RestStatus;
use actix_web::body::BoxBody;
use actix_web::{
    get,
    web::{self, Data},
    HttpRequest, HttpResponse, Responder, Scope,
};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use utoipa::IntoParams;
use utoipa::ToSchema;
use uuid::Uuid;

pub mod antrag;
pub mod sitzungen;

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
        .service(get_next_sitzung)
}

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct CreateTopParams {
    pub titel: String,
    pub sitzung_id: Uuid,
    pub inhalt: Option<serde_json::Value>,
    pub position: i64,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Person {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct TopWithAnträge {
    pub id: Uuid,
    pub position: i64,
    pub name: String,
    pub anträge: Vec<Antrag>,
    pub inhalt: Option<serde_json::Value>,
}

#[utoipa::path(
    path = "/api/topmanager/tops/{topid}/anträge/",
    params(("topid" = Uuid,Path,)),
    responses(
        (status = 201, description = "Created", body = TopWithAnträge),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/tops/{topid}/anträge/")]
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

#[utoipa::path(
    path = "/api/topmanager/current_tops/",
    responses(
        (status = 201, description = "Created", body = TopWithAnträge),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/current_tops/")]
async fn get_current_tops_with_anträge(db: Data<DatabasePool>) -> impl Responder {
    let tops_with_anträge: Option<anyhow::Result<Vec<TopWithAnträge>>> = db
        .transaction(move |mut transaction| async move {
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
        })
        .await
        .transpose();

    match tops_with_anträge {
        Some(tops_with_anträge) => RestStatus::ok_from_result(tops_with_anträge),
        None => RestStatus::NotFound,
    }
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/{id}/anträge/",
    responses(
        (status = 201, description = "Created", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/sitzung/{id}/anträge/")]
async fn anträge_by_sitzung(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let anträge = db
        .transaction(move |mut transaction| {
            let id = id.clone();
            async move {
                Ok((
                    transaction.anträge_by_sitzung(id.clone()).await?,
                    transaction,
                ))
            }
        })
        .await;

    RestStatus::ok_from_result(anträge)
}
