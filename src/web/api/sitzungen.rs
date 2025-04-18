use std::borrow::Cow;

use actix_web::web::{Data, Path};
use actix_web::{delete, get, patch, post, web, Responder, Scope};
use actix_web_validator::{Json as ActixJson, Query};
use chrono::{DateTime, Utc};
use icalendar::Calendar;
use serde::{Deserialize, Serialize};
use sqlx::Database;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::database::{DatabaseConnection, DatabaseTransaction};

use crate::domain::antrag_top_attachment_map::AntragTopMapping;
use crate::domain::calendar::{CalendarEvent, CalendarRepo};
use crate::domain::persons::{Abmeldung, Person};
use crate::domain::sitzung::{Sitzung, SitzungWithTops, Top, TopWithAnträge};
use crate::domain::templates::TemplatesRepo;
use crate::domain::{
    self,
    antrag_top_attachment_map::AntragTopAttachmentMap,
    sitzung::{SitzungKind, SitzungRepo, TopKind},
    Result,
};
use crate::web::calendar::CalendarData;
use crate::web::{auth, RestStatus};
use crate::TEMPLATE_ENGINE;

/// Create the sitzungs service under /sitzungen
pub(crate) fn service() -> Scope {
    let scope = web::scope("/sitzungen")
        .service(get_sitzungen)
        .service(post_sitzungen)
        .service(get_sitzungen_between)
        .service(get_sitzungen_after);

    // must come last
    register_sitzung_id_service(scope)
}

fn register_sitzung_id_service(parent: Scope) -> Scope {
    let scope = parent
        .service(get_sitzung_by_id)
        .service(patch_sitzung_by_id)
        .service(delete_sitzung_by_id)
        .service(get_abmeldungen_by_sitzung)
        .service(get_tops)
        .service(post_tops)
        .service(get_sitzung_template);

    // must come last
    register_top_id_service(scope)
}

fn register_top_id_service(parent: Scope) -> Scope {
    parent
        .service(patch_tops)
        .service(delete_tops)
        .service(assoc_antrag)
        .service(delete_assoc_antrag)
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct CreateSitzungParams {
    datetime: DateTime<Utc>,
    #[validate(length(min = 1))]
    location: String,
    kind: SitzungKind,
    antragsfrist: DateTime<Utc>,
    legislative_period: Uuid,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct CreateTopParams {
    #[validate(length(min = 1))]
    name: String,
    kind: TopKind,
    inhalt: String,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct UpdateSitzungParams {
    datetime: Option<DateTime<Utc>>,
    #[validate(length(min = 1))]
    location: Option<String>,
    kind: Option<SitzungKind>,
    antragsfrist: Option<DateTime<Utc>>,
    legislative_period: Option<Uuid>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct UpdateTopParams {
    #[validate(length(min = 1))]
    name: Option<String>,
    kind: Option<TopKind>,
    inhalt: Option<String>,
    weight: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct SitzungenAfterParams {
    timestamp: DateTime<Utc>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
#[validate(schema(function = "validate_sitzung_between_params"))]
pub struct SitzungBetweenParams {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct TemplateRenderStruct {
    sitzung: SitzungWithTops,
    persons: Vec<Person>,
    calendars: Vec<TemplateCalendar>,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct TemplateCalendar {
    name: String,
    events: Vec<CalendarEvent>,
}

fn validate_sitzung_between_params(
    params: &SitzungBetweenParams,
) -> core::result::Result<(), ValidationError> {
    if params.start > params.end {
        Err(ValidationError::new("sitzung_between_params")
            .with_message(Cow::Borrowed("start must be before end")))
    } else {
        Ok(())
    }
}

#[derive(Debug, Deserialize, IntoParams, ToSchema, Validate)]
pub struct AssocAntragParams {
    antrag_id: Uuid,
}

#[utoipa::path(
    path = "/api/sitzungen",
    responses(
        (status = 200, description = "Success", body = Vec<Sitzung>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("")]
async fn get_sitzungen(mut conn: DatabaseConnection) -> Result<impl Responder> {
    let result = conn.sitzungen().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen",
    request_body = CreateSitzungParams,
    responses(
        (status = 201, description = "Created", body = Sitzung),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("", wrap = "auth::capability::RequireManageSitzungen")]
async fn post_sitzungen(
    params: ActixJson<CreateSitzungParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .create_sitzung(
            params.datetime,
            params.location.as_str(),
            params.kind,
            params.antragsfrist,
            params.legislative_period,
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/after",
    params(SitzungenAfterParams),
    responses(
        (status = 200, description = "Success", body = SitzungWithTops),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/after")]
async fn get_sitzungen_after(
    params: Query<SitzungenAfterParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result =
        domain::sitzungen_after_with_tops(&mut *conn, params.timestamp, params.limit).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/between",
    params(SitzungBetweenParams),
    responses(
        (status = 200, description = "Success", body = Vec<Sitzung>),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/between")]
async fn get_sitzungen_between(
    params: Query<SitzungBetweenParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.sitzungen_between(params.start, params.end).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}",
    responses(
        (status = 200, description = "Success", body = SitzungWithTops),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{sitzung_id}")]
async fn get_sitzung_by_id(
    sitzung_id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = domain::sitzung_with_tops(&mut *conn, *sitzung_id).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}",
    request_body = UpdateSitzungParams,
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{sitzung_id}", wrap = "auth::capability::RequireManageSitzungen")]
async fn patch_sitzung_by_id(
    sitzung_id: Path<Uuid>,
    params: ActixJson<UpdateSitzungParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .update_sitzung(
            *sitzung_id,
            params.datetime,
            params.location.as_deref(),
            params.kind,
            params.antragsfrist,
            params.legislative_period,
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}",
    responses(
        (status = 200, description = "Success", body = Sitzung),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{sitzung_id}", wrap = "auth::capability::RequireManageSitzungen")]
async fn delete_sitzung_by_id(
    sitzung_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction.delete_sitzung(*sitzung_id).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/abmeldungen",
    responses(
        (status = 200, description = "Success", body = Vec<Abmeldung>),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{sitzung_id}/abmeldungen")]
async fn get_abmeldungen_by_sitzung(
    sitzung_id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    if conn.sitzung_by_id(*sitzung_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = domain::abmeldungen_by_sitzung(&mut *conn, *sitzung_id).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops",
    responses(
        (status = 200, description = "Success", body = Vec<TopWithAnträge>),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{sitzung_id}/tops")]
async fn get_tops(sitzung_id: Path<Uuid>, mut conn: DatabaseConnection) -> Result<impl Responder> {
    if conn.sitzung_by_id(*sitzung_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = domain::top_with_anträge_by_sitzung(&mut *conn, *sitzung_id).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops",
    request_body = CreateTopParams,
    responses(
        (status = 201, description = "Created", body = Top),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post(
    "/{sitzung_id}/tops",
    wrap = "auth::capability::RequireManageSitzungen"
)]
async fn post_tops(
    sitzung_id: Path<Uuid>,
    params: ActixJson<CreateTopParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    if transaction.sitzung_by_id(*sitzung_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = transaction
        .create_top(
            *sitzung_id,
            params.name.as_str(),
            params.inhalt.as_str(),
            params.kind,
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops/{top_id}",
    request_body = UpdateTopParams,
    responses(
        (status = 200, description = "Sucess", body = Top),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch(
    "/{sitzung_id}/tops/{top_id}",
    wrap = "auth::capability::RequireManageSitzungen"
)]
async fn patch_tops(
    path_params: Path<(Uuid, Uuid)>,
    params: ActixJson<UpdateTopParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (sitzung_id, top_id) = path_params.into_inner();

    if transaction.sitzung_by_id(sitzung_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = transaction
        .update_top(
            top_id,
            None, // we dont allow moving tops
            params.name.as_deref(),
            params.inhalt.as_deref(),
            params.kind,
            params.weight,
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops/{top_id}",
    responses(
        (status = 200, description = "Sucess", body = Top),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete(
    "/{sitzung_id}/tops/{top_id}",
    wrap = "auth::capability::RequireManageSitzungen"
)]
async fn delete_tops(
    path_params: Path<(Uuid, Uuid)>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (sitzung_id, top_id) = path_params.into_inner();

    if transaction.sitzung_by_id(sitzung_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = transaction.delete_top(top_id).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops/{top_id}/assoc",
    request_body = AssocAntragParams,
    responses(
        (status = 200, description = "Sucess", body = AntragTopMapping),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch(
    "/{sitzung_id}/tops/{top_id}/assoc",
    wrap = "auth::capability::RequireManageSitzungen"
)]
async fn assoc_antrag(
    path_params: Path<(Uuid, Uuid)>,
    params: ActixJson<AssocAntragParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (sitzung_id, top_id) = path_params.into_inner();

    if transaction.sitzung_by_id(sitzung_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    if transaction.top_by_id(top_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = transaction
        .attach_antrag_to_top(params.antrag_id, top_id)
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/tops/{top_id}/assoc",
    request_body = AssocAntragParams,
    responses(
        (status = 200, description = "Sucess", body = AntragTopMapping),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete(
    "/{sitzung_id}/tops/{top_id}/assoc",
    wrap = "auth::capability::RequireManageSitzungen"
)]
async fn delete_assoc_antrag(
    path_params: Path<(Uuid, Uuid)>,
    params: ActixJson<AssocAntragParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let (sitzung_id, top_id) = path_params.into_inner();

    if transaction.sitzung_by_id(sitzung_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    if transaction.top_by_id(top_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = transaction
        .detach_antrag_from_top(params.antrag_id, top_id)
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/sitzungen/{sitzung_id}/template/{name}",
    responses(
        (status = 200, description = "Sucess", body = String),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{sitzung_id}/template/{name}")]
async fn get_sitzung_template(
    path_params: Path<(Uuid, String)>,
    mut conn: DatabaseConnection,
    calendars: Data<CalendarData>,
) -> Result<impl Responder> {
    let (sitzung_id, template_name) = path_params.into_inner();

    let Some(sitzung) = domain::sitzung_with_tops(&mut *conn, sitzung_id).await? else {
        return Ok(RestStatus::NotFound);
    };

    let persons = domain::persons::PersonRepo::persons(&mut *conn).await?;

    let calendar_names = calendars.calendar_names();

    let mut calendars_events = Vec::new();
    for name in calendar_names {
        let Some(events) = calendars
            .calender_by_name(&Cow::Borrowed(name.as_str()))
            .await?
        else {
            return Ok(RestStatus::NotFound);
        };
        calendars_events.push(TemplateCalendar {
            name: name.to_string(),
            events,
        });
    }

    let Some(template) = conn.template_by_name(&template_name).await? else {
        return Ok(RestStatus::NotFound);
    };

    let result = TEMPLATE_ENGINE
        .read()
        .await
        .template(&template.name)
        .render(TemplateRenderStruct {
            sitzung,
            persons,
            calendars: calendars_events,
        })
        .to_string()?;

    Ok(RestStatus::Success(Some(result)))
}
