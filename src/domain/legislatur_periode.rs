use serde::Serialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::domain::sitzung::Sitzung;

use super::Result;

#[derive(Debug, Serialize, IntoParams, ToSchema, Clone)]
pub struct LegislaturPeriode {
    pub id: Uuid,
    pub name: String,
}

pub trait LegislaturPeriodeRepo {
    async fn create_legislatur_periode(&mut self, name: String) -> Result<LegislaturPeriode>;

    async fn legislatur_periode_by_id(&mut self, id: Uuid) -> Result<Option<LegislaturPeriode>>;

    async fn legislatur_perioden(&mut self) -> Result<Vec<LegislaturPeriode>>;

    async fn sitzungen_by_legislatur_perioden(&mut self, id: Uuid) -> Result<Vec<Sitzung>>;

    async fn update_legislatur_periode(
        &mut self,
        uuid: Uuid,
        name: String,
    ) -> Result<Option<LegislaturPeriode>>;

    async fn delete_legislatur_periode(&mut self, uuid: Uuid) -> Result<Option<LegislaturPeriode>>;
}
