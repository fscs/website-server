use chrono::{DateTime, Utc};
use utoipa::{IntoParams, ToSchema};

#[derive(serde::Serialize, Clone, IntoParams, ToSchema)]
pub struct CalendarEvent {
    pub summary: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}
