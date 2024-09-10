use actix_web::{delete, get, put, web, Responder, Scope};
use actix_web_validator::Json as ActixJson;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::{
    database::DatabaseTransaction,
    domain::persons::PersonRepo,
    web::{auth::User, RestStatus},
};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(get_roles)
        .service(create_role)
        .service(delete_role)
}

#[derive(Debug, IntoParams, Deserialize, ToSchema, Validate)]
pub struct RoleParams {
    #[validate(length(min = 1))]
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
    params(RoleParams),
    request_body = RoleParams,
    responses(
        (status = 200, description = "Success", body = Role),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[put("/")]
async fn create_role(
    _user: User,
    params: ActixJson<RoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.create_role(&params.name).await)
}

#[utoipa::path(
    path = "/api/roles/",
    params(RoleParams),
    request_body = RoleParams,
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/")]
async fn delete_role(
    _user: User,
    params: ActixJson<RoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(transaction.delete_role(&params.name).await)
}
