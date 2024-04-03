use crate::database::DatabasePool;
use crate::domain::TopManagerRepo;
use crate::web::topmanager::{CreateTopParams, RestStatus};
use actix_web::web::Data;
use actix_web::{get, put, web, Responder};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct CreateSitzungParams {
    pub datum: chrono::NaiveDateTime,
    pub name: String,
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/",
    params(CreateSitzungParams),
    responses(
        (status = 201, description = "Created", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/sitzung/")]
async fn get_sitzungen(db: Data<DatabasePool>) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| async move {
            Ok((transaction.get_sitzungen().await?, transaction))
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/",
    params(CreateSitzungParams),
    responses(
        (status = 201, description = "Created", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/sitzung/")]
async fn create_sitzung(
    db: Data<DatabasePool>,
    params: web::Json<CreateSitzungParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                Ok((
                    transaction
                        .create_sitzung(params.datum, params.name.as_str())
                        .await?,
                    transaction,
                ))
            }
        })
        .await;

    RestStatus::created_from_result(result)
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/{id}/",
    params(("id" = Uuid, Path,)),
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/sitzung/{sitzung_id}/top/")]
async fn create_top(
    db: Data<DatabasePool>,
    sitzung_id: web::Path<Uuid>,
    params: web::Json<CreateTopParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            let sitzung_id = sitzung_id.clone();
            async move {
                Ok((
                    transaction
                        .create_top(&params.titel, sitzung_id, params.inhalt)
                        .await?,
                    transaction,
                ))
            }
        })
        .await;

    RestStatus::created_from_result(result)
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/{id}/",
    params(("id" = Uuid, Path,)),
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/sitzung/{id}/tops/")]
async fn tops_by_sitzung(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let tops = db
        .transaction(move |mut transaction| {
            let id = id.clone();
            async move { Ok((transaction.tops_by_sitzung(id.clone()).await?, transaction)) }
        })
        .await;

    RestStatus::ok_from_result(tops)
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/next/",
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 404, description = "Not Found"),
    )
)]
#[get("/next_sitzung/")]
async fn get_next_sitzung(db: Data<DatabasePool>) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| async move {
            Ok((transaction.get_next_sitzung().await?, transaction))
        })
        .await;

    RestStatus::ok_or_not_found_from_result(result)
}
