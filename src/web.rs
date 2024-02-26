
pub(crate) mod calendar {

    use actix_web::{get, Responder};
    use chrono::{DateTime, NaiveTime, Utc};
    use icalendar::{Component,Event, EventLike};

    #[get("/calendar")]
    pub(crate) async fn get_events() -> impl Responder {
        let calendar = reqwest::get("https://nextcloud.inphima.de/remote.php/dav/public-calendars/CAx5MEp7cGrQ6cEe?export").await.unwrap().text().await.unwrap();
        let calendar = icalendar::parser::unfold(&calendar);
        let calendar = icalendar::parser::read_calendar(&calendar).unwrap();
        icalendar::Calendar::from(calendar).components.iter().filter_map(|component|
            match component {
                icalendar::CalendarComponent::Event(event) => Some(event),
                _ => None
            }
        ).filter_map(|event| {
                is_after(event, Utc::now())
            
        }).take(5)
        .map(|event| {
            format!("Event: {:?}, Ort: {:?}", event.get_summary(), event.get_location())
        })
        .collect::<Vec<_>>().join("\n")
    }

    fn dpt_to_date_time(date_perhaps_time: icalendar::DatePerhapsTime) -> Option<DateTime<Utc>> {
        match date_perhaps_time {
            icalendar::DatePerhapsTime::Date(date) => {
                Some(date.and_time(NaiveTime::MIN).and_utc())
            },
            icalendar::DatePerhapsTime::DateTime(naive_date_time) => {
                naive_date_time.try_into_utc()
            },
        }
    }

    fn is_after(event: &Event, date_time: DateTime<Utc>) -> Option<&Event> {
        if event.get_end().is_some_and(
            |dpt| 
            dpt_to_date_time(dpt).is_some_and(|end| end > date_time)
        ) {
            Some(event)
        } else {
            None
        }
    }
}