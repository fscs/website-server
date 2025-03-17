use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::Result;

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Attachment {
    pub id: Uuid,
    pub filename: String,
}

pub trait AttachmentRepo {
    async fn create_attachment(&mut self, filename: String) -> Result<Attachment>;

    async fn delete_attachment(&mut self, id: Uuid) -> Result<Option<Attachment>>;

    async fn attachment_by_id(&mut self, id: Uuid) -> Result<Option<Attachment>>;
}
