use actix_web::{delete, web};
use actix_web::{get, patch, put, web::Data, Responder, Scope};
use serde::Deserialize;
use sqlx::types::chrono;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::web::auth::User;
use crate::{database::DatabasePool, domain::PersonRepo, web::RestStatus};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(put_person_role)
        .service(get_persons)
        .service(get_person_by_role)
        .service(update_person)
        .service(create_person)
        .service(patch_person)
        .service(delete_person)
        .service(update_person_role)
        .service(delete_person_role)
}

#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct CreatePersonRoleParams {
    pub person_id: Uuid,
    pub rolle: String,
    pub anfangsdatum: chrono::NaiveDate,
    pub ablaufdatum: chrono::NaiveDate,
}

#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct UpdatePersonRoleParams {
    pub person_id: Uuid,
    pub rolle: String,
    pub anfangsdatum: chrono::NaiveDate,
    pub ablaufdatum: chrono::NaiveDate,
}

#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct DeletePersonRoleParams {
    pub person_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct CreatePersonParams {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct UpdatePersonParams {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct DeletePersonParams {
    pub id: Uuid,
}

#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct GetPersonsByRoleParams {
    pub rolle: String,
    pub anfangsdatum: chrono::NaiveDate,
    pub ablaufdatum: chrono::NaiveDate,
}

#[utoipa::path(
    path = "/api/person/role-mapping/",
    request_body = CreatePersonRoleParams,
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/role-mapping/")]
async fn put_person_role(
    user: User,
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonRoleParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction
                    .add_person_role_mapping(
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

#[utoipa::path(
    path = "/api/person/role-mapping/",
    request_body = UpdatePersonRoleParams,
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 400, description = "Bad Request"),
    )
)]
#[patch("/role-mapping/")]
async fn update_person_role(
    user: User,
    db: Data<DatabasePool>,
    params: web::Json<UpdatePersonRoleParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction
                    .update_person_role_mapping(
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

#[utoipa::path(
    path = "/api/person/role-mapping/",
    request_body = DeletePersonRoleParams,
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
    )
)]
#[delete("/role-mapping/")]
async fn delete_person_role(
    user: User,
    db: Data<DatabasePool>,
    params: web::Json<DeletePersonRoleParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction
                    .delete_person_role_mapping(params.person_id)
                    .await?;
                Ok((person, transaction))
            }
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[utoipa::path(
    path = "/api/person/",
    responses(
        (status = 200, description = "Success", body = Vec<Person>),
        (status = 400, description = "Bad Request"),
    )
)]
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

#[utoipa::path(
    path = "/api/person/",
    request_body = UpdatePersonParams,
    responses(
        (status = 200, description = "Success", body = Vec<Person>),
        (status = 400, description = "Bad Request"),
    )
)]
#[patch("/")]
async fn patch_person(
    user: User,
    db: Data<DatabasePool>,
    params: web::Json<UpdatePersonParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction.patch_person(params.id, &params.name).await?;
                Ok((person, transaction))
            }
        })
        .await;
    RestStatus::created_from_result(result)
}

#[utoipa::path(
    path = "/api/person/",
    request_body = DeletePersonParams,
    responses(
        (status = 200, description = "Success", body = Vec<Person>),
        (status = 400, description = "Bad Request"),
    )
)]
#[delete("/")]
async fn delete_person(
    user: User,
    db: Data<DatabasePool>,
    params: web::Json<DeletePersonParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let id = params.id;
            async move { Ok((transaction.delete_person(id).await?, transaction)) }
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[utoipa::path(
    path = "/api/person/",
    request_body = CreatePersonParams,
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 400, description = "Bad Request"),
    )
)]
#[put("/")]
async fn create_person(
    user: User,
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let person = transaction.create_person(&params.name).await?;
                Ok((person, transaction))
            }
        })
        .await;
    RestStatus::created_from_result(result)
}

#[utoipa::path(
    path = "/api/person/by-role/",
    request_body = GetPersonsByRoleParams,
    responses(
        (status = 200, description = "Success", body = Vec<Person>),
        (status = 400, description = "Bad Request"),
    )
)]
#[get("/by-role/")]
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

#[utoipa::path(
    path = "/api/person/role-mapping/",
    request_body = CreatePersonRoleParams,
    responses(
        (status = 200, description = "Success", body = Person),
        (status = 400, description = "Bad Request"),
    )
)]
#[patch("/role-mapping/")]
async fn update_person(
    user: User,
    db: Data<DatabasePool>,
    params: web::Json<CreatePersonRoleParams>,
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
