use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::antrag::Antrag;
use super::legislative_periods::LegislativePeriod;
use super::Result;

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
    Verschiedenes,
}

#[derive(Debug, Serialize, IntoParams, ToSchema, Clone)]
pub struct Sitzung {
    pub id: Uuid,
    pub datetime: DateTime<Utc>,
    pub location: String,
    pub kind: SitzungKind,
    pub antragsfrist: DateTime<Utc>,
    pub legislative_period: LegislativePeriod,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Top {
    pub id: Uuid,
    pub weight: i64,
    pub name: String,
    pub inhalt: String,
    pub kind: TopKind,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct SitzungWithTops {
    #[serde(flatten)]
    pub sitzung: Sitzung,
    pub tops: Vec<TopWithAnträge>,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct SitzungenWithTops {
    pub sitzungen: Vec<SitzungWithTops>,
}

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct TopWithAnträge {
    #[serde(flatten)]
    pub top: Top,
    pub anträge: Vec<Antrag>,
}

pub trait SitzungRepo {
    async fn create_sitzung(
        &mut self,
        datetime: DateTime<Utc>,
        location: &str,
        kind: SitzungKind,
        antragsfrist: DateTime<Utc>,
        legislative_period: Uuid,
    ) -> Result<Sitzung>;

    async fn create_top(
        &mut self,
        sitzung_id: Uuid,
        name: &str,
        inhalt: &str,
        kind: TopKind,
    ) -> Result<Top>;

    async fn sitzungen(&mut self) -> Result<Vec<Sitzung>>;

    async fn sitzung_by_id(&mut self, id: Uuid) -> Result<Option<Sitzung>>;

    async fn sitzungen_after(
        &mut self,
        datetime: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Sitzung>>;

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
        antragsfrist: Option<DateTime<Utc>>,
        legislative_period: Option<Uuid>,
    ) -> Result<Option<Sitzung>>;

    async fn update_top<'a>(
        &mut self,
        id: Uuid,
        sitzung_id: Option<Uuid>,
        name: Option<&'a str>,
        inhalt: Option<&'a str>,
        kind: Option<TopKind>,
        weight: Option<i64>,
    ) -> Result<Option<Top>>;

    async fn delete_sitzung(&mut self, id: Uuid) -> Result<Option<Sitzung>>;

    async fn delete_top(&mut self, id: Uuid) -> Result<Option<Top>>;
}
