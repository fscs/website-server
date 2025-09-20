use actix_web::web::Path;
use actix_web::{delete, get, patch, post, web, Responder, Scope};
use actix_web_validator::Json as ActixJson;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::database::DatabaseConnection;

use crate::domain::templates::{Template, TemplatesRepo};
use crate::domain::Result;
use crate::web::{cors_permissive, cors_restrictive, RestStatus};
use crate::TEMPLATE_ENGINE;

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct UpdateTemplateParams {
    inhalt: String,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct CreateTemplateParams {
    name: String,
    inhalt: String,
}

/// Create the template service under /templates
pub(crate) fn service() -> Scope {
    web::scope("/templates")
        .service(get_templates)
        .service(get_template_by_name)
        .service(delete_template)
        .service(patch_template)
        .service(create_template)
}

#[utoipa::path(
    path = "/api/templates",
    responses(
        (status = 200, description = "Success", body = Vec<Template>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("", wrap = "cors_permissive()")]
async fn get_templates(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.templates().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/templates/{name}",
    responses(
        (status = 200, description = "Success", body = Template),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{name}", wrap = "cors_permissive()")]
async fn get_template_by_name(
    name: Path<String>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.template_by_name(&name).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/templates/{template_name}",
    responses(
        (status = 200, description = "Success", body = Template),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{template_name}", wrap = "cors_restrictive()")]
async fn delete_template(
    name: Path<String>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.delete_template(&name).await?;

    TEMPLATE_ENGINE.write().await.remove_template(&name);

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/templates/{template_name}",
    request_body = UpdateTemplateParams,
    responses(
        (status = 200, description = "Success", body = Template),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{template_name}", wrap = "cors_restrictive()")]
async fn patch_template(
    name: Path<String>,
    params: ActixJson<UpdateTemplateParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.update_template(&name, &params.inhalt).await?;

    let mut write_handle = TEMPLATE_ENGINE.write().await;
    write_handle.remove_template(&name);
    write_handle.add_template(name.clone(), params.inhalt.clone())?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/templates",
    request_body = CreateTemplateParams,
    responses(
        (status = 200, description = "Success", body = Template),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("", wrap = "cors_restrictive()")]
async fn create_template(
    params: ActixJson<CreateTemplateParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn
        .create_template(Template {
            name: params.name.clone(),
            inhalt: params.inhalt.clone(),
        })
        .await?;

    TEMPLATE_ENGINE
        .write()
        .await
        .add_template(params.name.clone(), params.inhalt.clone())?;

    Ok(RestStatus::Success(Some(result)))
}
