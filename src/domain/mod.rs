use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, sqlx::Type, ToSchema, PartialEq, Eq)]
#[sqlx(type_name = "sitzungtype", rename_all = "lowercase")]
pub enum SitzungType {
    Normal,
    VV,
    WahlVV,
    Ersatz,
    Konsti,
    Dringlichkeit,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Sitzung {
    pub id: Uuid,
    pub datum: DateTime<Utc>,
    pub location: String,
    pub sitzung_type: SitzungType,
}

#[derive(Debug, Serialize, FromRow, IntoParams, ToSchema)]
pub struct Top {
    pub id: Uuid,
    pub weight: i64,
    pub name: String,
    pub inhalt: Option<serde_json::Value>,
    pub top_type: String,
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
    pub time: DateTime<Utc>,
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
pub trait SitzungRepo {
    async fn create_sitzung(
        &mut self,
        datetime: DateTime<Utc>,
        location: &str,
        sitzung_type: SitzungType,
    ) -> Result<Sitzung>;

    async fn create_top<'a>(
        &mut self,
        sitzung_id: Uuid,
        title: &str,
        top_type: &str,
        inhalt: Option<&'a serde_json::Value>,
    ) -> Result<Top>;

    async fn sitzungen(&mut self) -> Result<Vec<Sitzung>>;

    async fn sitzung_by_id(&mut self, id: Uuid) -> Result<Option<Sitzung>>;

    async fn first_sitzung_after(&mut self, datetime: DateTime<Utc>) -> Result<Option<Sitzung>>;

    async fn sitzungen_between(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Sitzung>>;

    async fn top_by_id(&mut self, id: Uuid) -> Result<Option<Top>>;

    async fn tops_by_sitzung(&mut self, sitzung_id: Uuid) -> Result<Vec<Top>>;

    async fn update_sitzung<'a>(
        &mut self,
        id: Uuid,
        datetime: Option<DateTime<Utc>>,
        location: Option<&'a str>,
        sitzung_type: Option<SitzungType>,
    ) -> Result<Sitzung>;

    async fn update_top<'a>(
        &mut self,
        id: Uuid,
        sitzung_id: Option<Uuid>,
        title: Option<&'a str>,
        top_type: Option<&'a str>,
        inhalt: Option<&'a serde_json::Value>,
    ) -> Result<Top>;

    async fn attach_antrag_to_top(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<AntragTopMapping>;

    async fn detach_antrag_from_top(&mut self, antrag_id: Uuid, top_id: Uuid) -> Result<()>;

    async fn delete_sitzung(&mut self, id: Uuid) -> Result<()>;

    async fn delete_top(&mut self, id: Uuid) -> Result<()>;
}

#[cfg_attr(test, automock)]
pub trait AntragRepo {
    async fn create_antrag(
        &mut self,
        creator: Uuid,
        title: &str,
        reason: &str,
        antragstext: &str,
    ) -> Result<Antrag>;

    async fn antrag_by_id(&mut self, id: Uuid) -> Result<Option<Antrag>>;

    async fn anträge_by_sitzung(&mut self, sitzung_id: Uuid) -> Result<Vec<Antrag>>;

    async fn anträge_by_top(&mut self, top_id: Uuid) -> Result<Vec<Antrag>>;

    async fn update_antrag<'a>(
        &mut self,
        id: Uuid,
        title: Option<&'a str>,
        reason: Option<&'a str>,
        antragstext: Option<&'a str>,
    ) -> Result<Antrag>;

    async fn delete_antrag(&mut self, id: Uuid) -> Result<()>;
}

#[cfg_attr(test, automock)]
pub trait DoorStateRepo {
    async fn create_doorstate(
        &mut self,
        timestamp: DateTime<Utc>,
        is_open: bool,
    ) -> Result<Doorstate>;

    async fn remove_doorstate(&mut self, timestamp: DateTime<Utc>) -> Result<()>;

    async fn doorstate_at(&mut self, timestamp: DateTime<Utc>) -> Result<Option<Doorstate>>;

    async fn doorstate_between(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Doorstate>>;
}

#[cfg_attr(test, automock)]
pub trait PersonRepo {
    async fn create_person(&mut self, name: &str) -> Result<Person>;

    async fn find_person(&mut self, id: Uuid) -> Result<Person>;

    async fn create_abmeldung(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Abmeldung>;

    async fn persons(&mut self) -> Result<Vec<Person>>;

    async fn update_person(&mut self, id: Uuid, name: &str) -> Result<Person>;

    async fn delete_person(&mut self, id: Uuid) -> Result<()>;

    async fn assign_role_to_person(
        &mut self,
        person_id: Uuid,
        role: &str,
        start: DateTime<Utc>,
        end: Option<DateTime<Utc>>,
    ) -> Result<PersonRoleMapping>;

    async fn delete_role_from_person(
        &mut self,
        person_id: Uuid,
        role: &str,
        start: DateTime<Utc>,
        end: Option<DateTime<Utc>>,
    ) -> Result<PersonRoleMapping>;

    async fn persons_with_role(
        &mut self,
        role: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Person>>;

    async fn abmeldungen_by_person(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<Abmeldung>>;

    async fn abmeldungen_between(
        &mut self,
        start: &NaiveDate,
        end: &NaiveDate,
    ) -> Result<Vec<Abmeldung>>;
}

// pub async fn get_tops_with_anträge(
//     sitzung: Uuid,
//     transaction: &mut DatabaseTransaction<'_>,
// ) -> Result<Vec<TopWithAnträge>> {
//     let tops = transaction.tops_by_sitzung(sitzung).await?;
//     let mut tops_with_anträge = vec![];
//     for top in tops {
//         let anträge = transaction.anträge_by_top(top.id).await?;
//         let top_with_anträge = TopWithAnträge {
//             id: top.id,
//             weight: top.weight,
//             name: top.name,
//             anträge,
//             inhalt: top.inhalt,
//             top_type: top.top_type,
//         };
//         tops_with_anträge.push(top_with_anträge);
//     }
//     Ok(tops_with_anträge)
// }
