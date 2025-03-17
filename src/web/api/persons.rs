use std::borrow::Cow;

use actix_web::web::Path;
use actix_web::{delete, put, web};
use actix_web::{get, patch, Responder, Scope};
use actix_web_validator::{Json as ActixJson, Query};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::database::{DatabaseConnection, DatabaseTransaction};
use crate::domain::persons::{Abmeldung, Person};
use crate::web::auth::User;
use crate::{
    domain::{persons::PersonRepo, Result},
    web::RestStatus,
};

/// Create the persons service under /persons
pub(crate) fn service() -> Scope {
    let scope = web::scope("/persons")
        .service(get_persons)
        .service(put_person)
        .service(get_persons_by_role)
        .service(get_person_by_user_name)
        .service(get_person_by_matrix_id);

    register_person_id_service(scope)
}

fn register_person_id_service(parent: Scope) -> Scope {
    parent
        .service(revoke_abmeldung)
        .service(revoke_role_from_person)
        .service(delete_person_by_id)
        .service(get_person_by_id)
        .service(patch_person)
        .service(add_role_to_person)
        .service(create_abmeldung)
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
    first_name: String,
    #[validate(length(min = 1))]
    last_name: String,
    #[validate(length(min = 1))]
    user_name: String,
    #[validate(length(min = 1))]
    matrix_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
pub struct UpdatePersonParams {
    #[validate(length(min = 1))]
    first_name: Option<String>,
    #[validate(length(min = 1))]
    last_name: Option<String>,
    #[validate(length(min = 1))]
    user_name: Option<String>,
    #[validate(length(min = 1))]
    matrix_id: Option<String>,
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

#[derive(Debug, Serialize, ToSchema, IntoParams, Validate)]
pub struct PublicPerson {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub user_name: Option<String>,
    pub matrix_id: Option<String>,
}

impl PublicPerson {
    pub fn public_from_person(person: Person) -> PublicPerson {
        PublicPerson {
            id: person.id,
            first_name: person.first_name,
            last_name: person.last_name,
            user_name: None,
            matrix_id: None,
        }
    }

    pub fn private_from_person(person: Person) -> PublicPerson {
        PublicPerson {
            id: person.id,
            first_name: person.first_name,
            last_name: person.last_name,
            user_name: Some(person.user_name),
            matrix_id: person.matrix_id,
        }
    }
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
    path = "/api/persons",
    responses(
        (status = 200, description = "Success", body = Vec<PublicPerson>),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("")]
async fn get_persons(user: Option<User>, mut conn: DatabaseConnection) -> Result<impl Responder> {
    let persons = conn.persons().await?;

    let result: Vec<_> = if user.is_some() {
        persons
            .into_iter()
            .map(PublicPerson::private_from_person)
            .collect()
    } else {
        persons
            .into_iter()
            .map(PublicPerson::public_from_person)
            .collect()
    };

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/persons/{person_id}",
    responses(
        (status = 200, description = "Success", body = PublicPerson),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{person_id}")]
async fn get_person_by_id(
    user: Option<User>,
    person_id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let person = conn.person_by_id(*person_id).await?;

    let result = person.map(|p| {
        if user.is_some() {
            PublicPerson::private_from_person(p)
        } else {
            PublicPerson::public_from_person(p)
        }
    });

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/persons",
    request_body = CreatePersonParams,
    responses(
        (status = 201, description = "Created", body = Person),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[put("")]
async fn put_person(
    _user: User,
    params: ActixJson<CreatePersonParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .create_person(
            params.first_name.as_str(),
            params.last_name.as_str(),
            params.user_name.as_str(),
            params.matrix_id.as_deref(),
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/persons/{person_id}",
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[delete("/{person_id}")]
async fn delete_person_by_id(
    _user: User,
    person_id: Path<Uuid>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction.delete_person(*person_id).await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/persons/{person_id}",
    request_body = UpdatePersonParams,
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[patch("/{person_id}")]
async fn patch_person(
    _user: User,
    person_id: Path<Uuid>,
    params: ActixJson<UpdatePersonParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    let result = transaction
        .update_person(
            *person_id,
            params.first_name.as_deref(),
            params.last_name.as_deref(),
            params.user_name.as_deref(),
            params.matrix_id.as_deref(),
        )
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/persons/by-role",
    params(PersonsByRoleParams),
    responses(
        (status = 200, description = "Success", body = Vec<PublicPerson>),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/by-role")]
async fn get_persons_by_role(
    user: Option<User>,
    params: Query<PersonsByRoleParams>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let persons = conn.persons_with_role(params.role.as_str()).await?;

    let result: Vec<_> = if user.is_some() {
        persons
            .into_iter()
            .map(PublicPerson::private_from_person)
            .collect()
    } else {
        persons
            .into_iter()
            .map(PublicPerson::public_from_person)
            .collect()
    };

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/persons/by-matrix-id/{matrix_id}",
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/by-matrix-id/{matrix_id}")]
async fn get_person_by_matrix_id(
    _user: User,
    matrix_id: Path<String>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.person_by_matrix_id(matrix_id.as_str()).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/persons/by-username/{user_name}",
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/by-username/{user_name}")]
async fn get_person_by_user_name(
    _user: User,
    user_name: Path<String>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    let result = conn.person_by_user_name(user_name.as_str()).await?;

    Ok(RestStatus::Success(result))
}

#[utoipa::path(
    path = "/api/persons/{person_id}/roles",
    responses(
        (status = 200, description = "Success", body = Vec<String>),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{person_id}/roles")]
async fn roles_by_person(
    person_id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    if conn.person_by_id(*person_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = conn.roles_by_person(*person_id).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/persons/{person_id}/roles",
    request_body = RoleParams,
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[put("/{person_id}/roles")]
async fn add_role_to_person(
    _user: User,
    person_id: Path<Uuid>,
    params: ActixJson<RoleParams>,
    mut transaction: DatabaseTransaction<'_>,
) -> Result<impl Responder> {
    if transaction.person_by_id(*person_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    transaction
        .assign_role_to_person(*person_id, params.role.as_str())
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(())))
}

#[utoipa::path(
    path = "/api/persons/{person_id}/roles",
    request_body = RoleParams,
    responses(
        (status = 200, description = "Success"),
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
) -> Result<impl Responder> {
    if transaction.person_by_id(*person_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    transaction
        .revoke_role_from_person(*person_id, params.role.as_str())
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(())))
}

#[utoipa::path(
    path = "/api/persons/{person_id}/abmeldungen",
    responses(
        (status = 200, description = "Success", body = Abmeldung),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/{person_id}/abmeldungen")]
async fn get_abmeldungen_by_person(
    _user: User,
    person_id: Path<Uuid>,
    mut conn: DatabaseConnection,
) -> Result<impl Responder> {
    if conn.person_by_id(*person_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = conn.abmeldungen_by_person(*person_id).await?;

    Ok(RestStatus::Success(Some(result)))
}

#[utoipa::path(
    path = "/api/persons/{person_id}/abmeldungen",
    request_body = AbmeldungParams,
    responses(
        (status = 201, description = "Created", body = Abmeldung),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Not Found"),
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
) -> Result<impl Responder> {
    if transaction.person_by_id(*person_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    let result = transaction
        .create_abmeldung(*person_id, params.start, params.end)
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Created(Some(result)))
}

#[utoipa::path(
    path = "/api/persons/{person_id}/abmeldungen",
    params(("person_id" = Uuid, Path, description = "person_id")),
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
) -> Result<impl Responder> {
    if transaction.person_by_id(*person_id).await?.is_none() {
        return Ok(RestStatus::NotFound);
    }

    transaction
        .revoke_abmeldung_from_person(*person_id, params.start, params.end)
        .await?;

    transaction.commit().await?;

    Ok(RestStatus::Success(Some(())))
}
