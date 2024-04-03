use actix_web::{delete, web};
use actix_web::{get, patch, put, web::Data, Responder, Scope};
use serde::Deserialize;
use sqlx::types::chrono;
use uuid::Uuid;

use crate::{database::DatabasePool, domain::AbmeldungRepo, web::RestStatus};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(put_person_abmeldung)
        .service(get_abmeldungen)
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePersonAbmeldungParams {
    pub person_id: Uuid,
    pub anfangsdatum: chrono::NaiveDate,
    pub ablaufdatum: chrono::NaiveDate,
}

#[put("/")]
async fn put_person_abmeldung(
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonAbmeldungParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction
                    .add_person_abmeldung(params.person_id, params.anfangsdatum, params.ablaufdatum)
                    .await?;
                Ok((person, transaction))
            }
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[get("/")]
async fn get_abmeldungen(db: Data<DatabasePool>) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| async move {
            let person = transaction.get_abmeldungen().await?;
            Ok((person, transaction))
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[patch("/")]
async fn update_person_abmeldung(
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonAbmeldungParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction
                    .update_person_abmeldung(
                        params.person_id,
                        params.anfangsdatum,
                        params.ablaufdatum,
                    )
                    .await?;
                Ok((person, transaction))
            }
        })
        .await;
    RestStatus::ok_from_result(result)
}

#[delete("/")]
async fn delete_person_abmeldung(
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonAbmeldungParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                transaction
                    .delete_person_abmeldung(
                        params.person_id,
                        params.anfangsdatum,
                        params.ablaufdatum,
                    )
                    .await?;
                Ok(((), transaction))
            }
        })
        .await;
    RestStatus::ok_from_result(result)
}
