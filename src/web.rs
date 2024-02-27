use actix_web::body::BoxBody;
use actix_web::{HttpRequest, HttpResponse, Responder};
use lazy_static::lazy_static;
use tera::Context;

pub(crate) mod calendar {
    use actix_web::{get, Responder};
    use actix_web::web::Json;
    use chrono::{DateTime, NaiveTime, Utc};
    use icalendar::{Component, Event, EventLike};
    use lazy_static::lazy_static;
    use crate::cache::TimedCache;
    use crate::web::TerraResponse;

    #[derive(serde::Serialize, Clone)]
    struct CalendarEvent {
        summary: Option<String>,
        location: Option<String>,
        description: Option<String>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    }

    #[get("/calendar")]
    pub(crate) async fn get_events() -> impl Responder {
        lazy_static! {
            static ref CACHE: TimedCache<Vec<CalendarEvent>>  = TimedCache::with_generator( || {
               request_calendar("https://nextcloud.inphima.de/remote.php/dav/public-calendars/CAx5MEp7cGrQ6cEe?export")
            }, std::time::Duration::from_secs(60 * 60 * 4));
        }
        let x = (*CACHE.get().await).clone();
        Json(x)
    }

    fn request_calendar(url: &str) -> Vec<CalendarEvent> {
        let calendar = reqwest::blocking::get(
            url,
        ).unwrap()
            .text()
            .unwrap();

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
        if event.get_end().is_some_and(|dpt| dpt_to_date_time(dpt).is_some_and(|end| end > date_time)) {
            Some(event)
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct TerraResponse {
    template: &'static str,
    context: Context,
}

impl Responder for TerraResponse {
    type Body = BoxBody;
    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        lazy_static! {
            static ref TERA: tera::Tera = {
                let current_dir = std::env::current_exe()
                    .unwrap()
                    .as_path()
                    .parent()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                tera::Tera::new(&(current_dir + "/templates/**/*")).unwrap()
            };
        }

        let body = TERA.render(self.template, &self.context).unwrap();
        HttpResponse::Ok().body(body)
    }


}