use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::Result;

#[derive(Debug, Serialize, ToSchema, PartialEq, Clone)]
pub struct AntragData {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begruendung: String,
    pub erstellt_am: DateTime<Utc>,
}

#[derive(Debug, Serialize, IntoParams, ToSchema, PartialEq)]
pub struct Antrag {
    #[serde(flatten)]
    pub data: AntragData,
    pub ersteller: Vec<Uuid>,
    pub anhaenge: Vec<Uuid>,
}

pub trait AntragRepo {
    async fn create_antrag(
        &mut self,
        ersteller: &[Uuid],
        titel: &str,
        begruendung: &str,
        antragstext: &str,
        erstellt_am: DateTime<Utc>,
    ) -> Result<Antrag>;

    async fn antraege(&mut self) -> Result<Vec<Antrag>>;

    async fn antrag_by_id(&mut self, id: Uuid) -> Result<Option<Antrag>>;

    async fn update_antrag<'a>(
        &mut self,
        id: Uuid,
        erstellt_am: DateTime<Utc>,
        ersteller: Option<&'a [Uuid]>,
        titel: Option<&'a str>,
        begruendung: Option<&'a str>,
        antragstext: Option<&'a str>,
    ) -> Result<Option<Antrag>>;

    async fn delete_antrag(&mut self, id: Uuid) -> Result<Option<AntragData>>;

    async fn add_anhang_to_antrag(&mut self, antrags_id: Uuid, anhang_id: Uuid) -> Result<()>;

    async fn delete_anhang_from_antrag(&mut self, antrags_id: Uuid, anhang_id: Uuid) -> Result<()>;
}
