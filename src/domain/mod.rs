use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Sitzung {
    pub id: Uuid,
    pub datum: NaiveDateTime,
    pub name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Antrag {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

pub trait SitzungRepo {

    async fn create_sitzung(&mut self, date_time: NaiveDateTime, name: &str) -> anyhow::Result<Sitzung>;

    async fn save_sitzung(&mut self, sitzung: Sitzung) -> anyhow::Result<Sitzung>;

    async fn find_sitzung_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Option<Sitzung>>;

    async fn find_sitzung_after(&mut self, date_time: NaiveDateTime) -> anyhow::Result<Option<Sitzung>>;
}

pub trait AntragRepo {
    async fn create_antrag(&mut self, titel: &str, antragstext: &str, begründung: &str) -> anyhow::Result<Antrag>;
}