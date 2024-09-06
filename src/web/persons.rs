use actix_web::web::{Path, Query};
use actix_web::{delete, post, put, web};
use actix_web::{get, patch, Responder, Scope};
use chrono::NaiveDate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::database::DatabaseTransaction;
use crate::{domain::person::PersonRepo, web::RestStatus};

// TODO: roles by person

pub(crate) fn service(path: &'static str) -> Scope {
    let scope = web::scope(path)
        .service(get_persons)
        .service(put_person)
        .service(get_persons_by_role);

    register_person_id_service(scope)
}

fn register_person_id_service(parent: Scope) -> Scope {
    parent
        .service(delete_person_by_id)
        .service(get_person_by_id)
        .service(patch_person)
        .service(add_role_to_person)
        .service(revoke_role_from_person)
        .service(create_abmeldung)
        .service(revoke_abmeldung)
        .service(get_abmeldungen_by_person)
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct PersonsByRoleParams {
    role: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct CreatePersonParams {
    name: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct UpdatePersonParams {
    name: Option<String>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct RoleParams {
    role: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct AbmeldungParams {
    start: NaiveDate,
    end: NaiveDate,
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
    path = "/api/persons/{person_id}/",
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{person_id}/")]
async fn get_person_by_id(
    person_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(transaction.person_by_id(*person_id).await)
}

#[utoipa::path(
    path = "/api/persons/",
    responses(
        (status = 201, description = "Created", body = Person),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[put("/")]
async fn put_person(
    params: web::Json<CreatePersonParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::created_from_result(transaction.create_person(params.name.as_str()).await)
}

#[utoipa::path(
    path = "/api/persons/{person_id}/",
    responses(
        (status = 200, description = "Success"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{person_id}/")]
async fn delete_person_by_id(
    person_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(transaction.delete_person(*person_id).await)
}

#[utoipa::path(
    path = "/api/persons/{person_id}/",
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{person_id}/")]
async fn patch_person(
    person_id: Path<Uuid>,
    params: web::Json<UpdatePersonParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(
        transaction
            .update_person(*person_id, params.name.as_deref())
            .await,
    )
}

#[utoipa::path(
    path = "/api/persons/by-role/",
    params(PersonsByRoleParams),
    responses(
        (status = 200, description = "Success", body = Vec<Person>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/by-role/")]
async fn get_persons_by_role(
    params: Query<PersonsByRoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.persons_with_role(&params.role).await)
}

#[utoipa::path(
    path = "/api/persons/{person_id}/roles",
    params(RoleParams),
    responses(
        (status = 200, description = "Success", body = PersonRoleMapping),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/{person_id}/roles")]
async fn add_role_to_person(
    person_id: Path<Uuid>,
    params: web::Json<RoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(
        transaction
            .assign_role_to_person(*person_id, params.role.as_str())
            .await,
    )
}

#[utoipa::path(
    path = "/api/persons/{person_id}/roles",
    params(RoleParams),
    responses(
        (status = 200, description = "Success", body = PersonRoleMapping),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{person_id}/roles")]
async fn revoke_role_from_person(
    person_id: Path<Uuid>,
    params: web::Json<RoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(
        transaction
            .revoke_role_from_person(*person_id, params.role.as_str())
            .await,
    )
}

#[utoipa::path(
    path = "/api/persons/{person_id}/abmeldungen",
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{person_id}/abmeldungen")]
async fn get_abmeldungen_by_person(
    person_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(
        transaction
            .abmeldungen_by_person(*person_id)
            .await,
    )
}

#[utoipa::path(
    path = "/api/persons/{person_id}/abmeldungen",
    params(AbmeldungParams),
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/{person_id}/abmeldungen")]
async fn create_abmeldung(
    person_id: Path<Uuid>,
    params: web::Json<AbmeldungParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::created_from_result(
        transaction
            .create_abmeldung(*person_id, params.start, params.end)
            .await,
    )
}

#[utoipa::path(
    path = "/api/persons/{person_id}/abmeldungen",
    params(AbmeldungParams),
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{person_id}/abmeldungen")]
async fn revoke_abmeldung(
    person_id: Path<Uuid>,
    params: web::Json<AbmeldungParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(
        transaction
            .revoke_abmeldung_from_person(*person_id, params.start, params.end)
            .await,
    )
}
