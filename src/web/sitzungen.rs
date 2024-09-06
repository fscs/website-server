use actix_web::{get, web, Responder, Scope};

use crate::database::DatabaseTransaction;

use crate::domain::sitzung::SitzungRepo;

use super::RestStatus;

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
}

#[utoipa::path(
    path = "/api/sitzungen/",
    responses(
        (status = 200, description = "Success", body = Vec<Sitzung>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/")]
async fn get_persons(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    RestStatus::ok_from_result(transaction.sitzungen().await)
}
