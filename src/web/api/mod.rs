use actix_web::{web, Scope};

pub(crate) mod antrag;
pub(crate) mod calendar;
pub(crate) mod door_state;
pub(crate) mod persons;
pub(crate) mod roles;
pub(crate) mod sitzungen;

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(calendar::service("/calendar"))
        .service(persons::service("/persons"))
        .service(roles::service("/roles"))
        .service(antrag::service("/anträge"))
        .service(door_state::service("/doorstate"))
        .service(sitzungen::service("/sitzungen"))
}
