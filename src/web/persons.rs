use std::borrow::Cow;

use actix_web::web::Path;
use actix_web::{delete, post, put, web};
use actix_web::{get, patch, Responder, Scope};
use actix_web_validator::{Query, Json as ActixJson};
use chrono::NaiveDate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::database::DatabaseTransaction;
use crate::web::auth::User;
use crate::{domain::persons::PersonRepo, web::RestStatus};

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
        .service(roles_by_person)
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct PersonsByRoleParams {
    #[validate(length(min = 1))]
    role: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct CreatePersonParams {
    #[validate(length(min = 1))]
    name: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct UpdatePersonParams {
    #[validate(length(min = 1))]
    name: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct RoleParams {
    #[validate(length(min = 1))]
    role: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
#[validate(schema(function = "validate_abmeldung_params"))]
pub struct AbmeldungParams {
    start: NaiveDate,
    end: NaiveDate,
}

fn validate_abmeldung_params(
    params: &AbmeldungParams,
) -> core::result::Result<(), ValidationError> {
    if params.start > params.end {
        Err(ValidationError::new("abmeldung_params")
            .with_message(Cow::Borrowed("start must be before end")))
    } else {
        Ok(())
    }
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
    request_body = CreatePersonParams,
    responses(
        (status = 201, description = "Created", body = Person),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[put("/")]
async fn put_person(
    _user: User,
    params: ActixJson<CreatePersonParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::created_from_result(transaction.create_person(params.name.as_str()).await)
}

#[utoipa::path(
    path = "/api/persons/{person_id}/",
    responses(
        (status = 200, description = "Success"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{person_id}/")]
async fn delete_person_by_id(
    _user: User,
    person_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(transaction.delete_person(*person_id).await)
}

#[utoipa::path(
    path = "/api/persons/{person_id}/",
    request_body = UpdatePersonParams,
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{person_id}/")]
async fn patch_person(
    _user: User,
    person_id: Path<Uuid>,
    params: ActixJson<UpdatePersonParams>,
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
        (status = 400, description = "Bad Request"),
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
    responses(
        (status = 200, description = "Success", body = Vec<String>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{person_id}/roles")]
async fn roles_by_person(
    person_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_from_result(transaction.roles_by_person(*person_id).await)
}

#[utoipa::path(
    path = "/api/persons/{person_id}/roles",
    params(RoleParams),
    request_body = RoleParams,
    responses(
        (status = 200, description = "Success", body = PersonRoleMapping),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/{person_id}/roles")]
async fn add_role_to_person(
    _user: User,
    person_id: Path<Uuid>,
    params: ActixJson<RoleParams>,
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
    request_body = RoleParams,
    responses(
        (status = 200, description = "Success", body = PersonRoleMapping),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{person_id}/roles")]
async fn revoke_role_from_person(
    _user: User,
    person_id: Path<Uuid>,
    params: ActixJson<RoleParams>,
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
    RestStatus::ok_from_result(transaction.abmeldungen_by_person(*person_id).await)
}

#[utoipa::path(
    path = "/api/persons/{person_id}/abmeldungen",
    params(AbmeldungParams),
    request_body = AbmeldungParams,
    responses(
        (status = 201, description = "Created", body = Abmeldung),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[put("/{person_id}/abmeldungen")]
async fn create_abmeldung(
    _user: User,
    person_id: Path<Uuid>,
    params: ActixJson<AbmeldungParams>,
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
    request_body = AbmeldungParams,
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{person_id}/abmeldungen")]
async fn revoke_abmeldung(
    _user: User,
    person_id: Path<Uuid>,
    params: ActixJson<AbmeldungParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> impl Responder {
    RestStatus::ok_or_not_found_from_result(
        transaction
            .revoke_abmeldung_from_person(*person_id, params.start, params.end)
            .await,
    )
}
