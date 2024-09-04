use actix_web::web::{Path, Query};
use actix_web::{delete, post, web};
use actix_web::{get, patch, Responder, Scope};
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

#[utoipa::path(
    path = "/api/persons/",
    responses(
        (status = 200, description = "Success", body = Vec<Person>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/")]
async fn get_persons(mut transaction: DatabaseTransaction<'_>) -> impl Responder {
    RestStatus::ok_from_result(transaction.persons().await)
}

#[utoipa::path(
    path = "/api/persons/{id}/",
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{id}/")]
async fn get_person_by_id(
    id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.person_by_id(*id).await)
}

#[utoipa::path(
    path = "/api/persons/{id}/",
    responses(
        (status = 200, description = "Success"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{id}/")]
async fn delete_person_by_id(
    id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.delete_person(*id).await)
}

#[utoipa::path(
    path = "/api/persons/{id}/",
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{id}/")]
async fn patch_person(
    id: Path<Uuid>,
    new_name: Option<String>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.update_person(*id, new_name.as_deref()).await)
}

#[derive(Deserialize)]
struct PersonsByRoleParams {
    role: String,
}

#[utoipa::path(
    path = "/api/persons/by-role/",
    responses(
        (status = 200, description = "Success", body = Vec<Person>),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/by-role/")]
async fn get_persons_by_role(
    param: Query<PersonsByRoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.persons_with_role(&param.role).await)
}

// TODO: person/{id}/roles/
//
// #[derive(Deserialize)]
// struct AddRoleToPersonParams {
//     role: String,
// }

// #[utoipa::path(
//     path = "/api/persons/{id}/roles/",
//     responses(
//         (status = 200, description = "Success", body = Vec<Role>),
//         (status = 404, description = "Not Found"),
//         (status = 500, description = "Internal Server Error"),
//     )
// )]
// #[post("/{id}/roles/")]
// async fn add_role_to_person(
//     id: Path<Uuid>,
//     param: web::Json<AddRoleToPersonParams>,
//     mut transaction: DatabaseTransaction<'_>,
// ) -> impl Responder {
//     RestStatus::created_from_result(transaction.assign_role_to_person(*id, &param.role).await)
// }

// #[derive(Deserialize)]
// struct DeleteRoleFromPersonParams {
//     role: String,
// }

// #[utoipa::path(
//     path = "/api/persons/{id}/roles/",
//     responses(
//         (status = 200, description = "Success"),
//         (status = 404, description = "Not Found"),
//         (status = 500, description = "Internal Server Error"),
//     )
// )]
// #[delete("/{id}/roles/")]
// async fn delete_role_from_person(
//     id: Path<Uuid>,
//     param: web::Json<DeleteRoleFromPersonParams>,
//     mut transaction: DatabaseTransaction<'_>,
// ) -> impl Responder {
//     RestStatus::created_from_result(transaction.revoke_role_from_person(*id, &param.role).await)
// }
