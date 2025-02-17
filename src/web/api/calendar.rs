use actix_web::web::{Data, Path};
use actix_web::{get, web, Responder, Scope};
use chrono::{DateTime, NaiveTime, Utc};
use icalendar::{Component, Event, EventLike};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::cache::TimedCache;
use crate::domain::calendar::CalendarEvent;
use crate::domain::{self, Result};
use crate::web::RestStatus;
use crate::ARGS;

type CalendarCacheMap = HashMap<String, TimedCache<Result<Vec<CalendarEvent>>>>;

pub(crate) fn app_data() -> CalendarCacheMap {
    let mut calendar_map: CalendarCacheMap = HashMap::new();

    for (name, url) in &ARGS.calendars {
        calendar_map.insert(
            name.clone(),
            TimedCache::with_generator(
                || request_calendar(url.as_str()),
                std::time::Duration::from_secs(60 * 60 * 4),
            ),
        );
    }

    calendar_map
}

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
async fn get_calendars() -> impl Responder {
    RestStatus::Success(Some(
        ARGS.calendars
            .iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>(),
    ))
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
    calendars: Data<CalendarCacheMap>,
) -> Result<impl Responder> {
    let Some(calendar_cache) = calendars.get(name.as_str()) else {
        return Ok(RestStatus::Success(None));
    };

    let Ok(ref calendar) = *calendar_cache.get().await else {
        return Err(domain::Error::Message(String::from(
            "failed to fetch calendar",
        )));
    };

    Ok(RestStatus::Success(Some(calendar.clone())))
}

fn request_calendar<'a>(
    url: &str,
) -> Pin<Box<dyn Future<Output = Result<Vec<CalendarEvent>>> + 'a>> {
    let url = url.to_owned();
    Box::pin(async { request_cal(url).await })
}

async fn request_cal(url: String) -> Result<Vec<CalendarEvent>> {
    let calendar = reqwest::get(&url).await?.text().await?;

    let calendar = icalendar::parser::unfold(&calendar);
    let calendar =
        icalendar::parser::read_calendar(&calendar).map_err(|s| domain::Error::Message(s))?;

    let mut events = icalendar::Calendar::from(calendar)
        .components
        .iter()
        .filter_map(|component| match component {
            icalendar::CalendarComponent::Event(event) => Some(event),
            _ => None,
        })
        .filter_map(|event| is_after(event, Utc::now()))
        .map(|event| CalendarEvent {
            summary: event.get_summary().map(std::string::ToString::to_string),
            location: event
                .get_location()
                .map(|m| m.to_string().replace('\\', "")),
            description: event
                .get_description()
                .map(std::string::ToString::to_string),
            start: event.get_start().and_then(dpt_to_date_time),
            end: event.get_start().and_then(dpt_to_date_time),
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
