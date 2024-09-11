use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::antrag::Antrag;
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
pub struct SitzungWithTops {
    #[serde(flatten)]
    pub sitzung: Sitzung,
    pub tops: Vec<TopWithAnträge>,
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
    ) -> Result<Option<Sitzung>>;

    async fn update_top<'a>(
        &mut self,
        id: Uuid,
        sitzung_id: Option<Uuid>,
        name: Option<&'a str>,
        inhalt: Option<&'a serde_json::Value>,
        kind: Option<TopKind>,
    ) -> Result<Option<Top>>;

    async fn delete_sitzung(&mut self, id: Uuid) -> Result<Option<Sitzung>>;

    async fn delete_top(&mut self, id: Uuid) -> Result<Option<Top>>;
}
