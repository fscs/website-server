use actix_web::web::{Path, Query};
use actix_web::{delete, post, web};
use actix_web::{get, patch, Responder, Scope};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::database::DatabaseTransaction;
use crate::{domain::PersonRepo, web::RestStatus};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(get_persons)
        .service(get_person_by_id)
        .service(delete_person_by_id)
        .service(patch_person)
        .service(get_persons_by_role)
}

#[utoipa::path()]
#[get("/")]
async fn get_persons(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    RestStatus::ok_from_result(transaction.persons().await)
}

#[utoipa::path()]
#[get("/{id}/")]
async fn get_person_by_id(
    id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.find_person(*id).await)
}

#[utoipa::path()]
#[delete("/{id}/")]
async fn delete_person_by_id(
    id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.delete_person(*id).await)
}

#[utoipa::path()]
#[patch("/{id}/")]
async fn patch_person(
    id: Path<Uuid>,
    new_name: String,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.update_person(*id, &new_name).await)
}

#[derive(Deserialize)]
struct PersonsByRoleParams {
    role: String,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[utoipa::path()]
#[get("/by-role")]
async fn get_persons_by_role(
    param: Query<PersonsByRoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(
        transaction
            .persons_with_role(&param.role, param.start, param.end)
            .await,
    )
}

#[derive(Deserialize)]
struct AddRoleToPersonParams {
    role: String,
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
}

#[utoipa::path()]
#[post("/{id}/roles/")]
async fn add_role_to_person(
    id: Path<Uuid>,
    param: Query<AddRoleToPersonParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::created_from_result(
        transaction
            .assign_role_to_person(&id, &param.role, param.start, param.end)
            .await,
    )
}

#[derive(Deserialize)]
struct DeleteRoleFromPersonParams {
    role: String,
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
}

#[utoipa::path()]
#[delete("/{id}/roles/")]
async fn delete_role_from_person(
    id: Path<Uuid>,
    param: Query<DeleteRoleFromPersonParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::created_from_result(
        transaction
            .delete_role_from_person(&id, &param.role, param.start, param.end)
            .await,
    )
}
