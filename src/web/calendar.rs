use std::{collections::HashMap, future::Future, pin::Pin};

use chrono::{DateTime, NaiveTime, Utc};
use icalendar::{Component, DatePerhapsTime, Event, EventLike};

use crate::{
    ARGS,
    cache::TimedCache,
    domain::{
        Error, Result,
        calendar::{CalendarEvent, CalendarRepo},
    },
};

pub(super) struct CalendarData {
    calendars: HashMap<String, TimedCache<Result<Vec<CalendarEvent>>>>,
}

impl CalendarData {
    pub fn new() -> Self {
        let mut calendar_map = HashMap::new();

        for (name, url) in &ARGS.calendars {
            calendar_map.insert(
                name.clone(),
                TimedCache::with_generator(
                    || request_calendar(url.as_str()),
                    std::time::Duration::from_secs(60 * 60 * 4),
                ),
            );
        }

        Self {
            calendars: calendar_map,
        }
    }
}

impl CalendarRepo for CalendarData {
    fn calendar_names(&self) -> Vec<String> {
        self.calendars.keys().map(|s| s.to_owned()).collect()
    }

    async fn calender_by_name(&self, name: &str) -> Result<Option<Vec<CalendarEvent>>> {
        let Some(calendar_cache) = self.calendars.get(name) else {
            return Ok(None);
        };

        let Ok(ref calendar) = *calendar_cache.get().await else {
            return Err(Error::Message(String::from("failed to fetch calendar")));
        };

        Ok(Some(calendar.clone()))
    }
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
    let calendar = icalendar::parser::read_calendar(&calendar).map_err(Error::Message)?;

    let mut events = icalendar::Calendar::from(calendar)
        .components
        .iter()
        .filter_map(|component| match component {
            icalendar::CalendarComponent::Event(event) => Some(event),
            _ => None,
        })
        .filter_map(|event| is_after(event, Utc::now()))
        .map(|event| CalendarEvent {
            summary: event.get_summary().map(|s| s.to_string()),
            location: event
                .get_location()
                .map(|m| m.to_string().replace('\\', "")),
            description: event.get_description().map(|s| s.to_string()),
            start: event.get_start().and_then(dpt_to_date_time),
            end: event.get_start().and_then(dpt_to_date_time),
        })
        .collect::<Vec<_>>();

    events.sort_by(|a, b| a.start.cmp(&b.start));
    Ok(events)
}

fn dpt_to_date_time(date_perhaps_time: icalendar::DatePerhapsTime) -> Option<DateTime<Utc>> {
    match date_perhaps_time {
        DatePerhapsTime::Date(date) => Some(date.and_time(NaiveTime::MIN).and_utc()),
        DatePerhapsTime::DateTime(naive_date_time) => naive_date_time.try_into_utc(),
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
