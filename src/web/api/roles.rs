use actix_web::{delete, get, put, web, Responder, Scope};
use actix_web_validator::Json as ActixJson;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::{
    database::{DatabaseConnection, DatabaseTransaction},
    domain::{
        persons::{PersonRepo, Role},
        Result,
    },
    web::{auth::User, RestStatus},
};

// Create the roles service under /roles
pub(crate) fn service() -> Scope {
    web::scope("/roles")
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
    path = "/api/roles",
    responses(
        (status = 200, description = "Success", body = Vec<Role>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("")]
async fn get_roles(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.roles().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/roles",
    request_body = RoleParams,
    responses(
        (status = 201, description = "Created", body = Role),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[put("")]
async fn create_role(
    _user: User,
    params: ActixJson<RoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    transaction.create_role(params.name.as_str()).await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(())))
}

#[utoipa::path(
    path = "/api/roles",
    request_body = RoleParams,
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("")]
async fn delete_role(
    _user: User,
    params: ActixJson<RoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction.delete_role(params.name.as_str()).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}
