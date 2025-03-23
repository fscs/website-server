use serde::Serialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::{Result, sitzung::SitzungWithTops};

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct LegislativePeriod {
    pub id: Uuid,
    pub name: String,
}

pub trait LegislativePeriodRepo {
    async fn create_legislative(&mut self, name: String) -> Result<LegislativePeriod>;

    async fn get_legislatives(&mut self) -> Result<Vec<LegislativePeriod>>;

    async fn get_legislatives_sitzungen(&mut self, id: Uuid) -> Result<Vec<SitzungWithTops>>;

    async fn patch_legislative(
        &mut self,
        uuid: Uuid,
        name: String,
    ) -> Result<Option<LegislativePeriod>>;

    async fn delete_legislative(&mut self, uuid: Uuid) -> Result<Option<LegislativePeriod>>;
}
