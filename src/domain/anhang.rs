use serde::Serialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::Result;

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct Anhang {
    pub id: Uuid,
    pub filename: String,
}

pub trait AnhangRepo {
    async fn create_anhang(&mut self, filename: String) -> Result<Anhang>;

    async fn delete_anhang(&mut self, id: Uuid) -> Result<Option<Anhang>>;

    async fn anhang_by_id(&mut self, id: Uuid) -> Result<Option<Anhang>>;
}
