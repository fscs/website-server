use chrono::{DateTime, Utc};
use utoipa::{IntoParams, ToSchema};

use super::Result;
use serde::Serialize;

#[derive(Debug, Serialize, Clone, IntoParams, ToSchema)]
pub struct CalendarEvent {
    pub summary: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

pub trait CalendarRepo {
    fn calendar_names(&self) -> Vec<String>;

    async fn calender_by_name(&self, name: &str) -> Result<Option<Vec<CalendarEvent>>>;
}
