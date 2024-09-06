use anyhow::Result;
use chrono::{DateTime, Utc};
#[cfg(test)]
use mockall::automock;
use serde::Serialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, IntoParams, ToSchema)]
pub struct DoorState {
    pub time: DateTime<Utc>,
    pub is_open: bool,
}

#[cfg_attr(test, automock)]
pub trait DoorStateRepo {
    async fn create_door_state(
        &mut self,
        timestamp: DateTime<Utc>,
        is_open: bool,
    ) -> Result<DoorState>;

    async fn door_state_at(&mut self, timestamp: DateTime<Utc>) -> Result<Option<DoorState>>;

    async fn door_state_between(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<DoorState>>;
}
