use serde::Serialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::Result;

#[derive(Debug, Serialize, IntoParams, ToSchema, PartialEq)]
pub struct AntragData {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[derive(Debug, Serialize, IntoParams, ToSchema, PartialEq)]
pub struct Antrag {
    #[serde(flatten)]
    pub data: AntragData,
    pub creators: Vec<Uuid>,
}

pub trait AntragRepo {
    async fn create_antrag(
        &mut self,
        creators: &[Uuid],
        title: &str,
        reason: &str,
        antragstext: &str,
    ) -> Result<Antrag>;

    async fn anträge(&mut self) -> Result<Vec<Antrag>>;

    async fn antrag_by_id(&mut self, id: Uuid) -> Result<Option<Antrag>>;

    async fn update_antrag<'a>(
        &mut self,
        id: Uuid,
        creators: Option<&'a [Uuid]>,
        title: Option<&'a str>,
        reason: Option<&'a str>,
        antragstext: Option<&'a str>,
    ) -> Result<Option<Antrag>>;

    async fn delete_antrag(&mut self, id: Uuid) -> Result<Option<AntragData>>;
}
