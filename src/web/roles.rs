use actix_web::{delete, get, post, web, Responder, Scope};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::{database::DatabaseTransaction, domain::person::PersonRepo, web::RestStatus};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(get_roles)
        .service(create_role)
        .service(delete_role)
}

#[derive(Debug, IntoParams, Deserialize, ToSchema)]
pub struct RoleParams {
    name: String,
}

#[utoipa::path(
    path = "/api/roles/",
    responses(
        (status = 200, description = "Success", body = Vec<Role>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/")]
async fn get_roles(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    RestStatus::ok_from_result(transaction.roles().await)
}

#[utoipa::path(
    path = "/api/roles/",
    responses(
        (status = 200, description = "Success", body = Role),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/")]
async fn create_role(
    params: web::Json<RoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.create_role(&params.name).await)
}

#[utoipa::path(
    path = "/api/roles/",
    responses(
        (status = 200, description = "Success"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/")]
async fn delete_role(
    params: web::Json<RoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.delete_role(&params.name).await)
}
