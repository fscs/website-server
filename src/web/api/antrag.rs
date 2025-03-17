use actix_http::header;
use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{
    delete, get, patch, post,
    web::{self, Path},
    HttpResponse, Responder, Scope,
};
use actix_web_validator::Json as ActixJson;
use log::debug;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::{
    database::{DatabaseConnection, DatabaseTransaction},
    domain::{
        self,
        antrag::{Antrag, AntragRepo},
        antrag_top_map::AntragTopMapRepo,
        attachment::AttachmentRepo,
        Result,
    },
    web::{auth::User, RestStatus},
    ARGS, UPLOAD_DIR,
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
    antragssteller: Vec<Uuid>,
    #[validate(length(min = 1))]
    begründung: String,
    #[validate(length(min = 1))]
    antragstext: String,
    #[validate(length(min = 1))]
    titel: String,
}

#[derive(Debug, IntoParams, Deserialize, ToSchema, Validate)]
pub struct UpdateAntragParams {
    antragssteller: Option<Vec<Uuid>>,
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
#[post("")]
async fn create_antrag(
    _user: User,
    params: ActixJson<CreateAntragParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .create_antrag(
            &params.antragssteller,
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
#[patch("/{antrag_id}")]
async fn patch_antrag(
    _user: User,
    params: ActixJson<UpdateAntragParams>,
    antrag_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .update_antrag(
            *antrag_id,
            params.antragssteller.as_deref(),
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
#[delete("/{antrag_id}")]
async fn delete_antrag(
    _user: User,
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
    _user: User,
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
#[delete("/{antrag_id}/attachments/{attachment_id}")]
async fn delete_antrag_attachment(
    _user: User,
    path_params: Path<(Uuid, Uuid)>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (antrag_id, attachment_id) = path_params.into_inner();
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
#[post("/{antrag_id}/attachments")]
async fn add_antrag_attachment(
    _user: User,
    antrag_id: Path<Uuid>,
    form: MultipartForm<UploadAntrag>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    if transaction.antrag_by_id(*antrag_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    match form.file.size {
        0 => {
            return Ok(RestStatus::BadRequest(
                "The Provided file was empty".to_string(),
            ))
        }
        length if length > usize::try_from(ARGS.max_file_size).unwrap() => {
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
            ))
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
