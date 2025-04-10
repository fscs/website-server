use actix_http::{StatusCode, header};
use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use actix_web::{
    HttpResponse, Responder, Scope, delete, get, patch, post,
    web::{self, Path},
};
use actix_web_validator::Json as ActixJson;
use log::debug;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::{
    ARGS, UPLOAD_DIR,
    database::{DatabaseConnection, DatabaseTransaction},
    domain::{
        self, Capability, Result,
        antrag::{Antrag, AntragRepo},
        antrag_top_attachment_map::AntragTopAttachmentMap,
        attachment::AttachmentRepo,
    },
    web::{
        RestStatus,
        auth::{self, User},
    },
};

/// Create the antrags service under /anträge
pub(crate) fn service() -> Scope {
    let scope = web::scope("/anträge")
        .service(get_anträge)
        .service(create_antrag)
        .service(get_orphan_anträge);

    // must come last
    register_antrag_id_service(scope)
}

fn register_antrag_id_service(parent: Scope) -> Scope {
    parent
        .service(get_antrag_attachment)
        .service(add_antrag_attachment)
        .service(delete_antrag_attachment)
        .service(get_antrag_by_id)
        .service(patch_antrag)
        .service(delete_antrag)
}

#[derive(Debug, IntoParams, Deserialize, ToSchema, Validate)]
pub struct CreateAntragParams {
    #[validate(length(min = 1))]
    begründung: String,
    #[validate(length(min = 1))]
    antragstext: String,
    #[validate(length(min = 1))]
    titel: String,
}

#[derive(Debug, IntoParams, Deserialize, ToSchema, Validate)]
pub struct UpdateAntragParams {
    #[validate(length(min = 1))]
    begründung: Option<String>,
    #[validate(length(min = 1))]
    antragstext: Option<String>,
    #[validate(length(min = 1))]
    titel: Option<String>,
}

#[derive(MultipartForm)]
pub struct UploadAntrag {
    file: TempFile,
}

#[utoipa::path(
    path = "/api/anträge",
    responses(
        (status = 200, description = "Success", body = Vec<Antrag>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("")]
async fn get_anträge(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.anträge().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/anträge/orphans",
    responses(
        (status = 200, description = "Success", body = Vec<Antrag>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/orphans")]
async fn get_orphan_anträge(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.orphan_anträge().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}",
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{antrag_id}")]
async fn get_antrag_by_id(
    antrag_id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.antrag_by_id(*antrag_id).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/anträge",
    request_body = CreateAntragParams,
    responses(
        (status = 201, description = "Created", body = Antrag),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("", wrap = "auth::capability::RequireCreateAntrag")]
async fn create_antrag(
    user: User,
    params: ActixJson<CreateAntragParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let person = user.query_person(&mut *transaction).await?;

    let result = transaction
        .create_antrag(
            &[person.id],
            &params.titel,
            &params.begründung,
            &params.antragstext,
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}",
    request_body = UpdateAntragParams,
    responses(
        (status = 200, description = "Success", body = Antrag),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{antrag_id}", wrap = "auth::capability::RequireCreateAntrag")]
async fn patch_antrag(
    user: User,
    params: ActixJson<UpdateAntragParams>,
    antrag_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let person = user.query_person(&mut *transaction).await?;

    if !user.has_capability(Capability::ManageAnträge) {
        let Some(antrag) = transaction.antrag_by_id(*antrag_id).await? else {
            return Ok(RestStatus::NotFound);
        };

        if !antrag.creators.contains(&person.id) {
            return Ok(RestStatus::Status(
                StatusCode::UNAUTHORIZED,
                "you are not a creator of this antrag".to_string(),
            ));
        }
    }

    let result = transaction
        .update_antrag(
            *antrag_id,
            None,
            params.titel.as_deref(),
            params.begründung.as_deref(),
            params.antragstext.as_deref(),
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}",
    responses(
        (status = 200, description = "Success"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{antrag_id}", wrap = "auth::capability::RequireManageAnträge")]
async fn delete_antrag(
    antrag_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction.delete_antrag(*antrag_id).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}/attachments/{attachment_id}",
    responses(
        (status = 200, description = "Success"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{antrag_id}/attachments/{attachment_id}")]
async fn get_antrag_attachment(
    path_params: Path<(Uuid, Uuid)>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let (_antrag_id, attachment_id) = path_params.into_inner();

    let Some(attachment) = conn.attachment_by_id(attachment_id).await? else {
        return Ok(HttpResponse::NotFound().finish());
    };

    let file_path = UPLOAD_DIR.as_path().join(attachment_id.to_string());

    debug!("Serving file: {:?}", file_path);

    Ok(HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, "application/octet-stream"))
        .append_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", attachment.filename),
        ))
        .body(std::fs::read(file_path)?))
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}/attachments/{attachment_id}",
    responses(
        (status = 200, description = "Success"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete(
    "/{antrag_id}/attachments/{attachment_id}",
    wrap = "auth::capability::RequireCreateAntrag"
)]
async fn delete_antrag_attachment(
    user: User,
    path_params: Path<(Uuid, Uuid)>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (antrag_id, attachment_id) = path_params.into_inner();

    if !user.has_capability(Capability::ManageAnträge) {
        let person = user.query_person(&mut *transaction).await?;

        let Some(antrag) = transaction.antrag_by_id(antrag_id).await? else {
            return Ok(RestStatus::NotFound);
        };

        if !antrag.creators.contains(&person.id) {
            return Ok(RestStatus::Status(
                StatusCode::UNAUTHORIZED,
                "you are not a creator of this antrag".to_string(),
            ));
        }
    }

    transaction
        .delete_attachment_from_antrag(antrag_id, attachment_id)
        .await?;

    let file_path = UPLOAD_DIR.as_path();

    std::fs::remove_file(file_path.join(attachment_id.to_string()))?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(())))
}

#[utoipa::path(
    path = "/api/anträge/{antrag_id}/attachments",
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post(
    "/{antrag_id}/attachments",
    wrap = "auth::capability::RequireCreateAntrag"
)]
async fn add_antrag_attachment(
    user: User,
    antrag_id: Path<Uuid>,
    form: MultipartForm<UploadAntrag>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let Some(antrag) = transaction.antrag_by_id(*antrag_id).await? else {
        return Ok(RestStatus::NotFound);
    };

    if !user.has_capability(Capability::ManageAnträge) {
        let person = user.query_person(&mut *transaction).await?;

        if !antrag.creators.contains(&person.id) {
            return Ok(RestStatus::Status(
                StatusCode::UNAUTHORIZED,
                "you are not a creator of this antrag".to_string(),
            ));
        }
    }

    match form.file.size {
        0 => {
            return Ok(RestStatus::BadRequest(
                "The Provided file was empty".to_string(),
            ));
        }
        length if length > ARGS.max_file_size => {
            return Ok(RestStatus::BadRequest(format!(
                "The uploaded file is too large. Maximum size is {} bytes.",
                ARGS.max_file_size
            )));
        }
        _ => {}
    };

    let temp_file_path = form.file.file.path();
    let file_name: &str = form
        .file
        .file_name
        .as_ref()
        .map(|m| m.as_ref())
        .unwrap_or("null");

    let file_path = UPLOAD_DIR.as_path();

    let attachment = transaction.create_attachment(file_name.to_string()).await;

    let attachment = match attachment {
        Ok(attachment) => attachment,
        Err(_) => {
            return Err(domain::Error::Message(
                "Could not create attachment".to_string(),
            ));
        }
    };

    transaction
        .add_attachment_to_antrag(*antrag_id, attachment.id)
        .await?;

    std::fs::copy(temp_file_path, file_path.join(attachment.id.to_string()))?;
    std::fs::remove_file(temp_file_path)?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(())))
}
