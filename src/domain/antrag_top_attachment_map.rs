use serde::Serialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::antrag::Antrag;
use super::sitzung::Top;
use super::Result;

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct AntragTopMapping {
    pub antrag_id: Uuid,
    pub top_id: Uuid,
}

pub trait AntragTopAttachmentMap {
    async fn anträge_by_top(&mut self, top_id: Uuid) -> Result<Vec<Antrag>>;

    async fn tops_by_antrag(&mut self, antrag_id: Uuid) -> Result<Vec<Top>>;

    async fn orphan_anträge(&mut self) -> Result<Vec<Antrag>>;

    async fn attach_antrag_to_top(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<Option<AntragTopMapping>>;

    async fn detach_antrag_from_top(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<Option<AntragTopMapping>>;
}
