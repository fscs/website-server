use actix_web::{web, Scope};

pub mod antrag;
pub mod sitzungen;

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path).service(antrag::service("/anträge"))
}
