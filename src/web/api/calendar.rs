use actix_web::web::{Data, Path};
use actix_web::{Responder, Scope, get, web};

use crate::domain::Result;
use crate::domain::calendar::{CalendarEvent, CalendarRepo};
use crate::web::RestStatus;
use crate::web::calendar::CalendarData;

// Create the calendar service under /calendar
pub(crate) fn service() -> Scope {
    web::scope("/calendar")
        .service(get_calendar_by_name)
        .service(get_calendars)
}

#[utoipa::path(
    path = "/api/calendar",
    responses(
        (status = 200, description = "Success", body = Vec<String>),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("")]
async fn get_calendars(calendars: Data<CalendarData>) -> impl Responder {
    RestStatus::Success(Some(calendars.calendar_names()))
}

#[utoipa::path(
    path = "/api/calendar/{calendar-name}",
    responses(
        (status = 200, description = "Success", body = CalendarEvent),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Not Found"),
    )
)]
#[get("/{name}")]
async fn get_calendar_by_name(
    name: Path<String>,
    calendars: Data<CalendarData>,
) -> Result<impl Responder> {
    Ok(RestStatus::Success(
        calendars.calender_by_name(name.as_str()).await?,
    ))
}
