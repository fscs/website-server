use actix_web::web;
use actix_web::{get, patch, put, web::Data, Responder, Scope};
use serde::Deserialize;
use sqlx::types::chrono;
use uuid::Uuid;

use crate::{database::DatabasePool, domain::PersonRepo, web::RestStatus};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(put_person_role)
        .service(get_persons)
        .service(get_person_by_role)
        .service(update_person)
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePersonParams {
    pub person_id: Uuid,
    pub rolle: String,
    pub anfangsdatum: chrono::NaiveDate,
    pub ablaufdatum: chrono::NaiveDate,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetPersonsByRoleParams {
    pub rolle: String,
    pub anfangsdatum: chrono::NaiveDate,
    pub ablaufdatum: chrono::NaiveDate,
}

#[put("/role-mapping")]
async fn put_person_role(
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction
                    .add_person(
                        params.person_id,
                        &params.rolle,
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

#[get("/")]
async fn get_persons(db: Data<DatabasePool>) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| async move {
            let person = transaction.get_persons().await?;
            Ok((person, transaction))
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[get("/by-role")]
async fn get_person_by_role(
    db: Data<DatabasePool>,
    params: web::Json<GetPersonsByRoleParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let persons = transaction
                    .get_person_by_role(&params.rolle, params.anfangsdatum, params.ablaufdatum)
                    .await?;
                Ok((persons, transaction))
            }
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[patch("/role-mapping")]
async fn update_person(
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction
                    .update_person(
                        params.person_id,
                        &params.rolle,
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