use serde::Serialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::{sitzung::Sitzung, Result};

#[derive(Debug, Serialize, IntoParams, ToSchema, Clone)]
pub struct LegislativePeriod {
    pub id: Uuid,
    pub name: String,
}

pub trait LegislativePeriodRepo {
    async fn create_legislative_period(&mut self, name: String) -> Result<LegislativePeriod>;

    async fn legislativ_period_by_id(&mut self, id: Uuid) -> Result<Option<LegislativePeriod>>;

    async fn legislative_periods(&mut self) -> Result<Vec<LegislativePeriod>>;

    async fn legislative_period_sitzungen(&mut self, id: Uuid) -> Result<Vec<Sitzung>>;

    async fn update_legislative_period(
        &mut self,
        uuid: Uuid,
        name: String,
    ) -> Result<Option<LegislativePeriod>>;

    async fn delete_legislative_period(&mut self, uuid: Uuid) -> Result<Option<LegislativePeriod>>;
}
