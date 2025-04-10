use actix_web::web::Path;
use actix_web::{delete, get, patch, web, Responder, Scope};
use actix_web_validator::Json as ActixJson;
use actix_web_validator::Query;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::database::DatabaseConnection;

use crate::domain::templates::{Template, TemplatesRepo};
use crate::domain::Result;
use crate::web::RestStatus;

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct TemplatesByNameParams {
    name: String,
}
#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct UpdateTemplateParams {
    inhalt: String,
}

/// Create the sitzungs service under /sitzungen
pub(crate) fn service() -> Scope {
    web::scope("/templates").service(get_templates)
}

#[utoipa::path(
    path = "/api/templates",
    responses(
        (status = 200, description = "Success", body = Vec<Template>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("")]
async fn get_templates(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.templates().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/templates/by-name",
    params(TemplatesByNameParams),
    responses(
        (status = 200, description = "Success", body = Template),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/by-name")]
async fn get_template_by_name(
    params: Query<TemplatesByNameParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.template_by_name(&params.name).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/templates/{template_name}",
    responses(
        (status = 200, description = "Success", body = Template),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{template_name}")]
async fn delete_template(
    name: Path<String>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.delete_template(&name).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/templates/{template_name}",
    responses(
        (status = 200, description = "Success", body = Template),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{template_name}")]
async fn patch_template(
    name: Path<String>,
    params: ActixJson<UpdateTemplateParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.update_template(&name, &params.inhalt).await?;

    Ok(RestStatus::Success(Some(result)))
}
