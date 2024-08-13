use crate::domain::TopManagerRepo;
use crate::web::auth::User;
use crate::web::topmanager::CreateTopParams;
use crate::{database::DatabaseTransaction, domain::get_tops_with_anträge};
use actix_web::{delete, get, patch, put, web, Responder};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct CreateSitzungParams {
    pub datum: chrono::NaiveDateTime,
    pub name: String,
    pub location: String,
}

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct GetSitzungByDateParams {
    pub datum: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct UpdateSitzungParams {
    pub id: Uuid,
    pub datum: chrono::NaiveDateTime,
    pub name: String,
    pub location: String,
}

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct DeleteSitzungParams {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct UpdateTopParams {
    pub id: Uuid,
    pub sitzung_id: Uuid,
    pub titel: String,
    pub inhalt: Option<serde_json::Value>,
    pub top_type: String,
}

#[derive(Debug, Deserialize, Clone, ToSchema, IntoParams)]
pub struct DeleteTopParams {
    pub id: Uuid,
}

#[utoipa::path(
    path = "/api/topmanager/sitzungen/",
    responses(
        (status = 201, description = "Created", body = Vec<Sitzung>),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/sitzungen/")]
async fn get_sitzungen(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    let result = transaction.get_sitzungen().await;

    transaction.rest_ok(result).await
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/{sitzung_id}/",
    params(("sitzung_id" = Uuid, Path,)),
    responses(
        (status = 201, description = "Created", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/sitzung/{sitzung_id}/")]
async fn get_sitzung(
    mut transaction: DatabaseTransaction<'_>,
    sitzung_id: web::Path<Uuid>,
) -> impl Responder {
    let result = transaction.get_sitzung(*sitzung_id).await;

    transaction.rest_ok(result).await
}
#[utoipa::path(
    path = "/api/topmanager/sitzung/",
    request_body = UpdateSitzungParams,
    responses(
        (status = 201, description = "Created", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[patch("/sitzung/")]
async fn update_sitzung(
    _user: User,
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<UpdateSitzungParams>,
) -> impl Responder {
    let result = transaction
        .update_sitzung(
            params.id,
            params.datum,
            params.name.as_str(),
            params.location.as_str(),
        )
        .await;
    transaction.rest_created(result).await
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/",
    request_body = DeleteSitzungParams,
    responses(
        (status = 201, description = "Created", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[delete("/sitzung/")]
async fn delete_sitzung(
    _user: User,
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<DeleteSitzungParams>,
) -> impl Responder {
    let result = transaction.delete_sitzung(params.id).await;
    transaction.rest_created(result).await
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/",
    request_body = CreateSitzungParams,
    responses(
        (status = 201, description = "Created", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/sitzung/")]
async fn create_sitzung(
    _user: User,
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<CreateSitzungParams>,
) -> impl Responder {
    let result = transaction
        .create_sitzung(params.datum, params.name.as_str(), params.location.as_str())
        .await;

    transaction.rest_created(result).await
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/{sitzung_id}/top/",
    params(("sitzung_id" = Uuid, Path,)),
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/sitzung/{sitzung_id}/top/")]
async fn create_top(
    _user: User,
    mut transaction: DatabaseTransaction<'_>,
    sitzung_id: web::Path<Uuid>,
    params: web::Json<CreateTopParams>,
) -> impl Responder {
    let result = transaction
        .create_top(&params.titel, *sitzung_id, &params.top_type, &params.inhalt)
        .await;
    transaction.rest_created(result).await
}

#[utoipa::path(
    path = "/api/topmanager/top/",
    responses(
        (status = 200, description = "Success", body = Top),
        (status = 400, description = "Bad Request"),
    )
)]
#[patch("/top/")]
async fn update_top(
    _user: User,
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<UpdateTopParams>,
) -> impl Responder {
    let result = transaction
        .update_top(
            params.sitzung_id,
            params.id,
            &params.titel,
            &params.top_type,
            &params.inhalt,
        )
        .await;

    transaction.rest_ok(result).await
}

#[utoipa::path(
    path = "/api/topmanager/top/",
    responses(
        (status = 200, description = "Success", body = Top),
        (status = 400, description = "Bad Request"),
    )
)]
#[delete("/top/")]
async fn delete_top(
    _user: User,
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<DeleteTopParams>,
) -> impl Responder {
    let result = transaction.delete_top(params.id).await;

    transaction.rest_ok(result).await
}

#[utoipa::path(
    path = "/api/topmanager/sitzung/{sitzung_id}/tops/",
    params(("sitzung_id" = Uuid, Path,)),
    responses(
        (status = 200, description = "Success", body = Vec<TopWithAnträge>),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/sitzung/{sitzung_id}/tops/")]
async fn tops_by_sitzung(
    mut transaction: DatabaseTransaction<'_>,
    id: web::Path<Uuid>,
) -> impl Responder {
    let tops = get_tops_with_anträge(id.clone(), &mut transaction).await;

    transaction.rest_ok(tops).await
}

#[utoipa::path(
    path = "/api/topmanager/next_sitzung/",
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 404, description = "Not Found"),
    )
)]
#[get("/next_sitzung/")]
async fn get_next_sitzung(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    let result = transaction.get_next_sitzung().await;

    transaction.rest_ok(result).await
}

#[utoipa::path(
    path = "/api/topmanager/sitzung_by_date/",
    request_body = GetSitzungByDateParams,
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 404, description = "Not Found"),
    )
)]
#[get("/sitzung_by_date/")]
async fn get_sitzung_by_date(
    mut transaction: DatabaseTransaction<'_>,
    params: web::Json<GetSitzungByDateParams>,
) -> impl Responder {
    let result = transaction.get_sitzung_by_date(params.datum).await;

    transaction.rest_ok(result).await
}
