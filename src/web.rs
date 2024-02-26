pub(crate) mod calendar {
    use actix_web::{get, Responder};
    use crate::web::calendar;

    #[get("/calendar")]
    pub(crate) async fn get_events() -> impl Responder {
        let calendar = reqwest::get("https://nextcloud.inphima.de/remote.php/dav/public-calendars/CAx5MEp7cGrQ6cEe?export").await.unwrap().text().await.unwrap();
        let calendar = icalendar::parser::unfold(&calendar);
        let calendar = icalendar::parser::read_calendar(&calendar).unwrap();
    }
}