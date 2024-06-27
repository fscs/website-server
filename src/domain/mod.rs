use chrono::{NaiveDate, NaiveDateTime};
#[cfg(test)]
use mockall::automock;
use serde::Serialize;
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Sitzung {
    pub id: Uuid,
    pub datum: NaiveDateTime,
    pub name: String,
    pub location: String,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Top {
    pub id: Uuid,
    pub weight: i64,
    pub name: String,
    pub inhalt: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Antrag {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Doorstate {
    pub time: NaiveDateTime,
    pub is_open: bool,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct PersonRoleMapping {
    pub person_id: Uuid,
    pub rolle: String,
    pub anfangsdatum: NaiveDate,
    pub ablaufdatum: NaiveDate,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Person {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Antragsstellende {
    pub antrags_id: Uuid,
    pub person_id: Uuid,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Abmeldung {
    pub person_id: Uuid,
    pub anfangsdatum: NaiveDate,
    pub ablaufdatum: NaiveDate,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct AntragTopMapping {
    pub antrag_id: Uuid,
    pub top_id: Uuid,
}

#[cfg_attr(test, automock)]
pub trait TopManagerRepo {
    async fn create_sitzung(
        &mut self,
        date_time: NaiveDateTime,
        name: &str,
        location: &str,
    ) -> anyhow::Result<Sitzung>;

    async fn create_person(&mut self, name: &str) -> anyhow::Result<Person>;

    async fn create_antragssteller(
        &mut self,
        antrag_id: Uuid,
        person_id: Uuid,
    ) -> anyhow::Result<()>;

    async fn save_sitzung(&mut self, sitzung: Sitzung) -> anyhow::Result<Sitzung>;

    async fn find_sitzung_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Option<Sitzung>>;

    async fn find_sitzung_after(
        &mut self,
        date_time: NaiveDateTime,
    ) -> anyhow::Result<Option<Sitzung>>;

    async fn get_sitzungen(&mut self) -> anyhow::Result<Vec<Sitzung>>;

    async fn create_antrag(
        &mut self,
        titel: &str,
        antragstext: &str,
        begründung: &str,
    ) -> anyhow::Result<Antrag>;

    async fn find_antrag_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Antrag>;

    async fn get_anträge(&mut self) -> anyhow::Result<Vec<Antrag>>;

    async fn delete_antrag(&mut self, uuid: Uuid) -> anyhow::Result<()>;

    async fn anträge_by_sitzung(&mut self, sitzung_id: Uuid) -> anyhow::Result<Vec<Antrag>>;

    async fn create_top(
        &mut self,
        titel: &str,
        sitzung_id: Uuid,
        inhalt: &Option<serde_json::Value>,
    ) -> anyhow::Result<Top>;

    async fn add_antrag_to_top(&mut self, antrag_id: Uuid, top_id: Uuid) -> anyhow::Result<()>;

    async fn anträge_by_top(&mut self, top_id: Uuid) -> anyhow::Result<Vec<Antrag>>;

    async fn tops_by_sitzung(&mut self, sitzung_id: Uuid) -> anyhow::Result<Vec<Top>>;

    async fn get_next_sitzung(&mut self) -> anyhow::Result<Option<Sitzung>>;

    async fn update_sitzung(
        &mut self,
        id: Uuid,
        datum: NaiveDateTime,
        name: &str,
    ) -> anyhow::Result<Sitzung>;

    async fn delete_sitzung(&mut self, id: Uuid) -> anyhow::Result<()>;

    async fn update_top(
        &mut self,
        sitzung_id: Uuid,
        id: Uuid,
        titel: &str,
        inhalt: &Option<serde_json::Value>,
    ) -> anyhow::Result<Top>;

    async fn delete_top(&mut self, id: Uuid) -> anyhow::Result<()>;

    async fn create_antrag_top_mapping(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> anyhow::Result<AntragTopMapping>;

    async fn delete_antrag_top_mapping(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> anyhow::Result<()>;

    async fn get_sitzung(&mut self, top_id: Uuid) -> anyhow::Result<Option<Sitzung>>;
}

#[cfg_attr(test, automock)]
pub trait DoorStateRepo {
    async fn add_doorstate(
        &mut self,
        time: NaiveDateTime,
        state: bool,
    ) -> anyhow::Result<Doorstate>;
    async fn get_doorstate(&mut self, time: NaiveDateTime) -> anyhow::Result<Option<Doorstate>>;
    async fn get_doorstate_history(&mut self) -> anyhow::Result<Option<Vec<Doorstate>>>;
}

#[cfg_attr(test, automock)]
pub trait PersonRepo {
    async fn patch_person(&mut self, id: Uuid, name: &str) -> anyhow::Result<Person>;

    async fn add_person_role_mapping(
        &mut self,
        person_id: Uuid,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> anyhow::Result<PersonRoleMapping>;

    async fn update_person_role_mapping(
        &mut self,
        person_id: Uuid,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> anyhow::Result<PersonRoleMapping>;

    async fn delete_person_role_mapping(&mut self, person_id: Uuid) -> anyhow::Result<()>;

    async fn create_person(&mut self, name: &str) -> anyhow::Result<Person>;

    async fn get_persons(&mut self) -> anyhow::Result<Vec<Person>>;

    async fn get_person_by_role(
        &mut self,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> anyhow::Result<Vec<Person>>;

    async fn update_person(
        &mut self,
        person_id: Uuid,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> anyhow::Result<PersonRoleMapping>;

    async fn delete_person(&mut self, id: Uuid) -> anyhow::Result<()>;
}

#[cfg_attr(test, automock)]
pub trait AbmeldungRepo {
    async fn add_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> anyhow::Result<Abmeldung>;

    async fn get_abmeldungen(&mut self) -> anyhow::Result<Vec<Abmeldung>>;

    async fn update_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> anyhow::Result<Abmeldung>;

    async fn delete_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> anyhow::Result<()>;

    async fn get_abmeldungen_next_sitzung(&mut self) -> anyhow::Result<Vec<Abmeldung>>;
}
