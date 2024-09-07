use crate::cache::TimedCache;

use actix_web::web::Json;
use actix_web::{get, web, HttpResponseBuilder, Responder, Scope};
use anyhow::anyhow;
use chrono::{DateTime, NaiveTime, Utc};
use icalendar::{Component, Event, EventLike};
use lazy_static::lazy_static;
use reqwest::StatusCode;
use std::future::Future;
use std::pin::Pin;
use utoipa::{IntoParams, ToSchema};

#[derive(serde::Serialize, Clone, IntoParams, ToSchema)]
pub struct CalendarEvent {
    summary: Option<String>,
    location: Option<String>,
    description: Option<String>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
}

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(get_events)
        .service(get_branchen_events)
}

#[utoipa::path(
    path = "/api/calendar/",
    responses(
        (status = 200, description = "Success", body = CalendarEvent),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/")]
async fn get_events() -> impl Responder {
    lazy_static! {
        static ref CACHE: TimedCache<anyhow::Result<Vec<CalendarEvent>>> =
            TimedCache::with_generator(
                || {
                    request_calendar("https://nextcloud.inphima.de/remote.php/dav/public-calendars/CAx5MEp7cGrQ6cEe?export")
                },
                std::time::Duration::from_secs(60 * 60 * 4)
            );
    }
    let Ok(ref x) = *CACHE.get().await else {
        return HttpResponseBuilder::new(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Could not access Calendar");
    };
    HttpResponseBuilder::new(StatusCode::OK).json(Json(x.clone()))
}

#[utoipa::path(
    path = "/api/calendar/branchen/",
    responses(
        (status = 200, description = "Success", body = CalendarEvent),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/branchen/")]
async fn get_branchen_events() -> impl Responder {
    lazy_static! {
        static ref CACHE: TimedCache<anyhow::Result<Vec<CalendarEvent>>> =
            TimedCache::with_generator(
                || {
                    request_calendar("https://nextcloud.inphima.de/remote.php/dav/public-calendars/CKpykNdtKHkA6Z9B?export")
                },
                std::time::Duration::from_secs(60 * 60)
            );
    }
    let Ok(ref x) = *CACHE.try_get().await else {
        return HttpResponseBuilder::new(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Could not access Calendar");
    };
    HttpResponseBuilder::new(StatusCode::OK).json(Json(x.clone()))
}

fn request_calendar<'a>(
    url: &str,
) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<CalendarEvent>>> + 'a>> {
    let url = url.to_owned();
    Box::pin(async { request_cal(url).await })
}

async fn request_cal(url: String) -> anyhow::Result<Vec<CalendarEvent>> {
    let calendar = reqwest::get(&url).await?.text().await?;

    let calendar = icalendar::parser::unfold(&calendar);
    let calendar = icalendar::parser::read_calendar(&calendar).map_err(|e| anyhow!("{:?}", e))?;

    let mut events = icalendar::Calendar::from(calendar)
        .components
        .iter()
        .filter_map(|component| match component {
            icalendar::CalendarComponent::Event(event) => Some(event),
            _ => None,
        })
        .filter_map(|event| is_after(event, Utc::now()))
        .filter_map(|event| {
            Some(CalendarEvent {
                summary: event.get_summary().map(std::string::ToString::to_string),
                location: event
                    .get_location()
                    .map(|m| m.to_string().replace('\\', "")),
                description: event.get_description().map(std::string::ToString::to_string),
                start: event.get_start().and_then(dpt_to_date_time),
                end: event.get_start().and_then(dpt_to_date_time),
            })
        })
        .collect::<Vec<_>>();

    events.sort_by(|a, b| a.start.cmp(&b.start));
    Ok(events)
}

fn dpt_to_date_time(date_perhaps_time: icalendar::DatePerhapsTime) -> Option<DateTime<Utc>> {
    match date_perhaps_time {
        icalendar::DatePerhapsTime::Date(date) => Some(date.and_time(NaiveTime::MIN).and_utc()),
        icalendar::DatePerhapsTime::DateTime(naive_date_time) => naive_date_time.try_into_utc(),
    }
}

fn is_after(event: &Event, date_time: DateTime<Utc>) -> Option<&Event> {
    if event
        .get_end()
        .is_some_and(|dpt| dpt_to_date_time(dpt).is_some_and(|end| end > date_time))
    {
        Some(event)
    } else {
        None
    }
}
