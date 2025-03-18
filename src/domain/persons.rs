use chrono::NaiveDate;
use serde::Serialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::Result;

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct PersonRoleMapping {
    pub person_id: Uuid,
    pub rolle: String,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Person {
    pub id: Uuid,
    pub full_name: String,
    pub user_name: String,
    pub matrix_id: Option<String>,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Abmeldung {
    pub person_id: Uuid,
    pub anfangsdatum: NaiveDate,
    pub ablaufdatum: NaiveDate,
}

pub trait PersonRepo {
    async fn create_person(
        &mut self,
        full_name: &str,
        user_name: &str,
        matrix_id: Option<&str>,
    ) -> Result<Person>;

    async fn create_role(&mut self, name: &str) -> Result<()>;

    async fn create_abmeldung(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Abmeldung>;

    async fn persons(&mut self) -> Result<Vec<Person>>;

    async fn roles(&mut self) -> Result<Vec<String>>;

    async fn roles_by_person(&mut self, id: Uuid) -> Result<Vec<String>>;

    async fn person_by_id(&mut self, id: Uuid) -> Result<Option<Person>>;

    async fn person_by_matrix_id(&mut self, matrix_id: &str) -> Result<Option<Person>>;

    async fn person_by_user_name(&mut self, user_name: &str) -> Result<Option<Person>>;

    async fn persons_with_role(&mut self, role: &str) -> Result<Vec<Person>>;

    async fn abmeldungen_by_person(&mut self, person_id: Uuid) -> Result<Vec<Abmeldung>>;

    async fn abmeldungen_at(&mut self, date: NaiveDate) -> Result<Vec<Abmeldung>>;

    async fn assign_role_to_person(&mut self, person_id: Uuid, role: &str) -> Result<()>;

    async fn revoke_role_from_person(&mut self, person_id: Uuid, role: &str) -> Result<()>;

    async fn revoke_abmeldung_from_person(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<()>;

    async fn update_person<'a>(
        &mut self,
        id: Uuid,
        full_name: Option<&str>,
        user_name: Option<&str>,
        matrix_id: Option<&str>,
    ) -> Result<Option<Person>>;

    async fn delete_person(&mut self, id: Uuid) -> Result<Option<Person>>;

    async fn delete_role(&mut self, name: &str) -> Result<Option<String>>;
}
