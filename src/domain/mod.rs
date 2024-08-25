use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime};
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{database::DatabaseTransaction, web::topmanager::TopWithAnträge};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, sqlx::Type, ToSchema)]
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
    pub datum: NaiveDateTime,
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
        location: &str,
        sitzung_type: SitzungType,
    ) -> Result<Sitzung>;

    async fn create_person(&mut self, name: &str) -> Result<Person>;

    async fn create_antragssteller(&mut self, antrag_id: Uuid, person_id: Uuid) -> Result<()>;

    async fn save_sitzung(&mut self, sitzung: Sitzung) -> Result<Sitzung>;

    async fn find_sitzung_by_id(&mut self, uuid: Uuid) -> Result<Option<Sitzung>>;

    async fn find_sitzung_after(
        &mut self,
        date_time: NaiveDateTime,
    ) -> anyhow::Result<Option<Sitzung>>;

    async fn get_sitzungen(&mut self) -> Result<Vec<Sitzung>>;

    async fn create_antrag(
        &mut self,
        titel: &str,
        antragstext: &str,
        begründung: &str,
    ) -> Result<Antrag>;

    async fn find_antrag_by_id(&mut self, uuid: Uuid) -> Result<Antrag>;

    async fn get_anträge(&mut self) -> Result<Vec<Antrag>>;

    async fn delete_antrag(&mut self, uuid: Uuid) -> Result<()>;

    async fn anträge_by_sitzung(&mut self, sitzung_id: Uuid) -> Result<Vec<Antrag>>;

    async fn create_top(
        &mut self,
        titel: &str,
        sitzung_id: Uuid,
        top_type: &str,
        inhalt: &Option<serde_json::Value>,
    ) -> Result<Top>;

    async fn add_antrag_to_top(&mut self, antrag_id: Uuid, top_id: Uuid) -> Result<()>;

    async fn anträge_by_top(&mut self, top_id: Uuid) -> Result<Vec<Antrag>>;

    async fn tops_by_sitzung(&mut self, sitzung_id: Uuid) -> Result<Vec<Top>>;

    async fn get_next_sitzung(&mut self) -> Result<Option<Sitzung>>;

    async fn get_sitzung_by_date(&mut self, date: NaiveDateTime) -> Result<Option<Sitzung>>;

    async fn update_sitzung(
        &mut self,
        id: Uuid,
        datum: NaiveDateTime,
        location: &str,
        sitzung_type: SitzungType,
    ) -> Result<Sitzung>;

    async fn delete_sitzung(&mut self, id: Uuid) -> Result<()>;

    async fn update_top(
        &mut self,
        sitzung_id: Uuid,
        id: Uuid,
        titel: &str,
        top_type: &str,
        inhalt: &Option<serde_json::Value>,
    ) -> Result<Top>;

    async fn delete_top(&mut self, id: Uuid) -> Result<()>;

    async fn create_antrag_top_mapping(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<AntragTopMapping>;

    async fn delete_antrag_top_mapping(&mut self, antrag_id: Uuid, top_id: Uuid) -> Result<()>;

    async fn get_sitzung(&mut self, top_id: Uuid) -> Result<Option<Sitzung>>;
}

#[cfg_attr(test, automock)]
pub trait DoorStateRepo {
    async fn add_doorstate(
        &mut self,
        time: NaiveDateTime,
        state: bool,
    ) -> anyhow::Result<Doorstate>;

    async fn get_doorstate(&mut self, time: NaiveDateTime) -> Result<Option<Doorstate>>;

    async fn get_doorstate_history(&mut self) -> Result<Option<Vec<Doorstate>>>;
}

#[cfg_attr(test, automock)]
pub trait PersonRepo {
    async fn patch_person(&mut self, id: Uuid, name: &str) -> Result<Person>;

    async fn add_person_role_mapping(
        &mut self,
        person_id: Uuid,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<PersonRoleMapping>;

    async fn update_person_role_mapping(
        &mut self,
        person_id: Uuid,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<PersonRoleMapping>;

    async fn delete_person_role_mapping(&mut self, person_id: Uuid) -> Result<()>;

    async fn create_person(&mut self, name: &str) -> Result<Person>;

    async fn get_persons(&mut self) -> Result<Vec<Person>>;

    async fn get_person_by_role(
        &mut self,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<Vec<Person>>;

    async fn update_person(
        &mut self,
        person_id: Uuid,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<PersonRoleMapping>;

    async fn delete_person(&mut self, id: Uuid) -> Result<()>;
}

pub async fn get_tops_with_anträge(
    sitzung: Uuid,
    transaction: &mut DatabaseTransaction<'_>,
) -> Result<Vec<TopWithAnträge>> {
    let tops = transaction.tops_by_sitzung(sitzung).await?;
    let mut tops_with_anträge = vec![];
    for top in tops {
        let anträge = transaction.anträge_by_top(top.id).await?;
        let top_with_anträge = TopWithAnträge {
            id: top.id,
            weight: top.weight,
            name: top.name,
            anträge,
            inhalt: top.inhalt,
            top_type: top.top_type,
        };
        tops_with_anträge.push(top_with_anträge);
    }
    Ok(tops_with_anträge)
}

#[cfg_attr(test, automock)]
pub trait AbmeldungRepo {
    async fn add_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<Abmeldung>;

    async fn get_abmeldungen(&mut self) -> Result<Vec<Abmeldung>>;

    async fn update_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<Abmeldung>;

    async fn delete_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<()>;

    async fn get_abmeldungen_between(
        &mut self,
        start: &NaiveDate,
        end: &NaiveDate,
    ) -> Result<Vec<Abmeldung>>;
}
