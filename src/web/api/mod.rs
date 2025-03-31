use actix_web::{Scope, web};

pub(crate) mod antrag;
pub(crate) mod calendar;
pub(crate) mod door_state;
pub(crate) mod legislative_periods;
pub(crate) mod persons;
pub(crate) mod roles;
pub(crate) mod sitzungen;

/// Create the API Service under /api
pub(crate) fn service() -> Scope {
    web::scope("/api")
        .service(calendar::service())
        .service(persons::service())
        .service(roles::service())
        .service(antrag::service())
        .service(door_state::service())
        .service(sitzungen::service())
        .service(legislative_periods::service())
}
