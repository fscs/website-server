use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, sqlx::Type, ToSchema, PartialEq, Eq)]
#[sqlx(type_name = "sitzungkind", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SitzungKind {
    Normal,
    VV,
    WahlVV,
    Ersatz,
    Konsti,
    Dringlichkeit,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, sqlx::Type, ToSchema, PartialEq, Eq)]
#[sqlx(type_name = "topkind", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TopKind {
    Regularia,
    Bericht,
    Normal,
    Verschiedenes
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Sitzung {
    pub id: Uuid,
    pub datetime: DateTime<Utc>,
    pub location: String,
    pub kind: SitzungKind,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Top {
    pub id: Uuid,
    pub weight: i64,
    pub name: String,
    pub inhalt: Option<serde_json::Value>,
    pub kind: TopKind,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct AntragData {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Antrag {
    #[serde(flatten)]
    pub data: AntragData,
    pub creators: Vec<Uuid>,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct SitzungWithTops {
    #[serde(flatten)]
    pub sitzung: Sitzung,
    pub tops: Vec<TopWithAnträge>
}


#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct TopWithAnträge {
    #[serde(flatten)]
    pub top: Top,
    pub anträge: Vec<Antrag>
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct DoorState {
    pub time: DateTime<Utc>,
    pub is_open: bool,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct PersonRoleMapping {
    pub person_id: Uuid,
    pub rolle: String,
    pub anfangsdatum: NaiveDate,
    pub ablaufdatum: NaiveDate,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Person {
    pub id: Uuid,
    pub name: String,
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

#[derive(Debug, Serialize, IntoParams, ToSchema)]
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
        kind: SitzungKind,
    ) -> Result<Sitzung>;

    async fn create_top<'a>(
        &mut self,
        sitzung_id: Uuid,
        name: &str,
        inhalt: Option<&'a serde_json::Value>,
        kind: TopKind,
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
        kind: Option<SitzungKind>,
    ) -> Result<Sitzung>;

    async fn update_top<'a>(
        &mut self,
        id: Uuid,
        sitzung_id: Option<Uuid>,
        name: Option<&'a str>,
        inhalt: Option<&'a serde_json::Value>,
        kind: Option<TopKind>,
    ) -> Result<Top>;

    async fn delete_sitzung(&mut self, id: Uuid) -> Result<()>;

    async fn delete_top(&mut self, id: Uuid) -> Result<()>;
}

#[cfg_attr(test, automock)]
pub trait AntragTopMapRepo {
    async fn anträge_by_top(&mut self, top_id: Uuid) -> Result<Vec<Antrag>>;
    
    async fn attach_antrag_to_top(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<AntragTopMapping>;

    async fn detach_antrag_from_top(&mut self, antrag_id: Uuid, top_id: Uuid) -> Result<()>;
}

#[cfg_attr(test, automock)]
pub trait AntragRepo {
    async fn create_antrag(
        &mut self,
        creators: &[Uuid],
        title: &str,
        reason: &str,
        antragstext: &str,
    ) -> Result<Antrag>;

    async fn antrag_by_id(&mut self, id: Uuid) -> Result<Option<Antrag>>;

    async fn update_antrag<'a>(
        &mut self,
        id: Uuid,
        creators: Option<&'a [Uuid]>,
        title: Option<&'a str>,
        reason: Option<&'a str>,
        antragstext: Option<&'a str>,
    ) -> Result<Antrag>;

    async fn delete_antrag(&mut self, id: Uuid) -> Result<()>;
}

#[cfg_attr(test, automock)]
pub trait DoorStateRepo {
    async fn create_door_state(
        &mut self,
        timestamp: DateTime<Utc>,
        is_open: bool,
    ) -> Result<DoorState>;

    async fn door_state_at(&mut self, timestamp: DateTime<Utc>) -> Result<Option<DoorState>>;

    async fn door_state_between(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<DoorState>>;
}

#[cfg_attr(test, automock)]
pub trait PersonRepo {
    async fn create_person(&mut self, name: &str) -> Result<Person>;

    async fn create_role(&mut self, name: &str) -> Result<()>;

    async fn create_abmeldung(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Abmeldung>;

    async fn persons(&mut self) -> Result<Vec<Person>>;
    
    async fn roles(&mut self) -> Result<Vec<String>>;
    
    async fn person_by_id(&mut self, id: Uuid) -> Result<Option<Person>>;

    async fn persons_with_role(
        &mut self,
        role: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<Person>>;

    async fn abmeldungen_by_person(
        &mut self,
        person_id: Uuid,
    ) -> Result<Vec<Abmeldung>>;

    async fn abmeldungen_at(
        &mut self,
        date: NaiveDate,
    ) -> Result<Vec<Abmeldung>>;
    
    async fn assign_role_to_person(
        &mut self,
        person_id: Uuid,
        role: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<PersonRoleMapping>;

    async fn revoke_role_from_person(
        &mut self,
        person_id: Uuid,
        role: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<()>;

    async fn revoke_abmeldung_from_person(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<()>;
    
    async fn update_person<'a>(&mut self, id: Uuid, name: Option<&'a str>) -> Result<Person>;

    async fn delete_person(&mut self, id: Uuid) -> Result<()>;
    
    async fn delete_role(&mut self, name: &str) -> Result<()>;
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
