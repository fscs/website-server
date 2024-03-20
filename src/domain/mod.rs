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
pub struct Top {
    pub id: Uuid,
    pub position: i64,
    pub name: String,
    pub inhalt: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Antrag {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

pub trait TopManagerRepo {

    async fn create_sitzung(&mut self, date_time: NaiveDateTime, name: &str) -> anyhow::Result<Sitzung>;

    async fn save_sitzung(&mut self, sitzung: Sitzung) -> anyhow::Result<Sitzung>;

    async fn find_sitzung_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Option<Sitzung>>;

    async fn find_sitzung_after(&mut self, date_time: NaiveDateTime) -> anyhow::Result<Option<Sitzung>>;

    async fn get_sitzungen(&mut self) -> anyhow::Result<Vec<Sitzung>>;

    async fn create_antrag(&mut self, titel: &str, antragstext: &str, begründung: &str) -> anyhow::Result<Antrag>;

    async fn find_antrag_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Antrag>;

    async fn get_anträge(&mut self) -> anyhow::Result<Vec<Antrag>>;

    async fn delete_antrag(&mut self, uuid: Uuid) -> anyhow::Result<()>;

    async fn anträge_by_sitzung(&mut self, sitzung_id: Uuid) -> anyhow::Result<Vec<Antrag>>;

    async fn create_top(&mut self, titel: &str, sitzung_id: Uuid, inhalt: Option<serde_json::Value>) -> anyhow::Result<Top>;

    async fn add_antrag_to_top(&mut self, antrag_id: Uuid, top_id: Uuid) -> anyhow::Result<()>;

    async fn anträge_by_top(&mut self, top_id: Uuid) -> anyhow::Result<Vec<Antrag>>;

    async fn tops_by_sitzung(&mut self, sitzung_id: Uuid) -> anyhow::Result<Vec<Top>>;
}