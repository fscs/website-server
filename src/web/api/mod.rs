use actix_web::{web, Scope};

pub(crate) mod antrag;
pub(crate) mod calendar;
pub(crate) mod legislative_periods;
pub(crate) mod persons;
pub(crate) mod roles;
pub(crate) mod sitzungen;
pub(crate) mod templates;

/// Create the API Service under /api
pub(crate) fn service() -> Scope {
    web::scope("/api")
        .service(calendar::service())
        .service(persons::service())
        .service(roles::service())
        .service(antrag::service())
        .service(sitzungen::service())
        .service(legislative_periods::service())
        .service(templates::service())
}
