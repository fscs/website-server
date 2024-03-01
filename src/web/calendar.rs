use crate::cache::TimedCache;
use actix_web::web::Json;
use actix_web::{get, web, Responder, Scope};
use chrono::{DateTime, NaiveTime, Utc};
use icalendar::{Component, Event, EventLike};
use lazy_static::lazy_static;
use std::future::Future;
use std::pin::Pin;

#[derive(serde::Serialize, Clone)]
struct CalendarEvent {
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

#[get("/")]
async fn get_events() -> impl Responder {
    lazy_static! {
        static ref CACHE: TimedCache<Vec<CalendarEvent>> = TimedCache::with_generator(
            || {
                request_calendar("https://nextcloud.inphima.de/remote.php/dav/public-calendars/CAx5MEp7cGrQ6cEe?export")
            },
            std::time::Duration::from_secs(60 * 60 * 4)
        );
    }
    let x = (*CACHE.get().await).clone();
    Json(x)
}

#[get("/branchen")]
async fn get_branchen_events() -> impl Responder {
    lazy_static! {
        static ref CACHE: TimedCache<Vec<CalendarEvent>> = TimedCache::with_generator(
            || {
                request_calendar("https://nextcloud.inphima.de/remote.php/dav/public-calendars/CKpykNdtKHkA6Z9B?export")
            },
            std::time::Duration::from_secs(60 * 60 * 4)
        );
    }
    let x = (*CACHE.get().await).clone();
    Json(x)
}

fn request_calendar<'a>(url: &str) -> Pin<Box<dyn Future<Output = Vec<CalendarEvent>> + 'a>> {
    let url = url.to_owned();
    Box::pin((move || async { request_cal(url).await })())
}

async fn request_cal(url: String) -> Vec<CalendarEvent> {
    let calendar = reqwest::get(&url).await.unwrap().text().await.unwrap();

    let calendar = icalendar::parser::unfold(&calendar);
    let calendar = icalendar::parser::read_calendar(&calendar).unwrap();

    icalendar::Calendar::from(calendar)
        .components
        .iter()
        .filter_map(|component| match component {
            icalendar::CalendarComponent::Event(event) => Some(event),
            _ => None,
        })
        .filter_map(|event| is_after(event, Utc::now()))
        .filter_map(|event| {
            Some(CalendarEvent {
                summary: event.get_summary().map(|m| m.to_string()),
                location: event.get_location().map(|m| m.to_string()),
                description: event.get_description().map(|m| m.to_string()),
                start: event.get_start().map(|d| dpt_to_date_time(d)).flatten(),
                end: event.get_start().map(|d| dpt_to_date_time(d)).flatten(),
            })
        })
        .collect::<Vec<_>>()
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
