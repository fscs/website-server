use std::future::Future;
use std::ops::{Deref, DerefMut};
use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;
use crate::domain::{Antrag, Sitzung, Top, TopManagerRepo};

#[derive(Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

#[must_use]
#[derive(Debug)]
pub struct DatabaseTransaction<'a> {
    transaction: Transaction<'a, Postgres>,
}

impl<'a> DerefMut for DatabaseTransaction<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.transaction
    }
}

impl<'a> Deref for DatabaseTransaction<'a> {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &*self.transaction
    }
}

impl DatabaseTransaction<'_> {

    pub async fn commit(self) -> anyhow::Result<()> {
        self.transaction.commit().await?;
        Ok(())
    }

    pub async fn rollback(self) -> anyhow::Result<()> {
        self.transaction.rollback().await?;
        Ok(())
    }
}

impl DatabasePool {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

        Ok(DatabasePool { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn start_transaction(&self) -> anyhow::Result<DatabaseTransaction<'static>> {
        Ok(DatabaseTransaction {
            transaction: self.pool.begin().await?,
        })
    }

    pub async fn transaction<'a, T: 'static, Fut: Future<Output=anyhow::Result<(T, DatabaseTransaction<'a>)>>, F: Fn(DatabaseTransaction<'a>) -> Fut + 'static>(& self, fun: F) -> anyhow::Result<T> {
            let transaction = self.start_transaction().await?;
            let (result, transaction) = fun(transaction).await?;
            transaction.commit().await?;
            Ok(result)
    }
}

impl TopManagerRepo for DatabaseTransaction<'_> {

    async fn create_sitzung(&mut self, date_time: NaiveDateTime, name: &str) -> anyhow::Result<Sitzung> {
        Ok(sqlx::query_as!(Sitzung, "INSERT INTO sitzungen (datum, name) VALUES ($1, $2) RETURNING *", date_time, name)
            .fetch_one(&mut **self)
            .await?)
    }

    async fn save_sitzung(&mut self, sitzung: Sitzung) -> anyhow::Result<Sitzung> {
        Ok(sqlx::query_as!(Sitzung, "UPDATE sitzungen SET datum = $1, name = $2 WHERE id = $3 RETURNING *", sitzung.datum, sitzung.name, sitzung.id)
            .fetch_one(&mut **self)
            .await?)
    }

    async fn find_sitzung_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Option<Sitzung>> {
        Ok(sqlx::query_as!(Sitzung,  "SELECT * FROM sitzungen WHERE id = $1", uuid).fetch_optional(&mut **self).await?)
    }

    async fn find_sitzung_after(&mut self, date_time: NaiveDateTime) -> anyhow::Result<Option<Sitzung>> {
        Ok(sqlx::query_as!(Sitzung, "SELECT * FROM sitzungen WHERE datum > $1", date_time)
            .fetch_optional(&mut **self)
            .await?)
    }

    async fn get_sitzungen(&mut self) -> anyhow::Result<Vec<Sitzung>> {
        Ok(sqlx::query_as!(Sitzung, "SELECT sitzungen.id, sitzungen.datum, sitzungen.name FROM sitzungen JOIN tops ON sitzungen.id = tops.sitzung_id")
            .fetch_all(&mut **self)
            .await?)
    }

    async fn create_antrag(&mut self, titel: &str, antragstext: &str, begründung: &str) -> anyhow::Result<Antrag> {
        Ok(sqlx::query_as!(Antrag, "INSERT INTO anträge (titel, antragstext, begründung) VALUES ($1, $2, $3) RETURNING *", titel, antragstext, begründung)
            .fetch_one(&mut **self)
            .await?)
    }

    async fn find_antrag_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Antrag> {
        Ok(sqlx::query_as!(Antrag, "SELECT * FROM anträge WHERE id = $1", uuid)
            .fetch_one(&mut **self)
            .await?)
    }


    async fn get_anträge(&mut self) -> anyhow::Result<Vec<Antrag>> {
        Ok(sqlx::query_as!(Antrag, "SELECT * FROM anträge")
            .fetch_all(&mut **self)
            .await?)
    }

    async fn delete_antrag(&mut self, uuid: Uuid) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM antragstop WHERE antrag_id = $1", uuid)
            .execute(&mut **self)
            .await?;

        sqlx::query!("DELETE FROM anträge WHERE id = $1", uuid)
            .execute(&mut **self)
            .await?;
        Ok(())
    }

    async fn anträge_by_sitzung(&mut self, sitzung_id: Uuid) -> anyhow::Result<Vec<Antrag>> {
        Ok(sqlx::query_as!(Antrag,
            "SELECT anträge.id, anträge.antragstext, anträge.begründung, anträge.titel FROM anträge
                  JOIN antragstop ON anträge.id = antragstop.antrag_id
                  JOIN tops ON antragstop.top_id = tops.id WHERE tops.sitzung_id = $1", sitzung_id)
            .fetch_all(&mut **self)
            .await?)
    }

    async fn create_top(&mut self, titel: &str, sitzung_id: Uuid, inhalt: Option<Value>) -> anyhow::Result<Top> {
        let position = sqlx::query!("SELECT COUNT(*) FROM tops WHERE sitzung_id = $1", sitzung_id)
            .fetch_one(&mut **self)
            .await?
            .count;

        Ok(sqlx::query_as!(Top, "INSERT INTO tops (name, sitzung_id, position, inhalt) VALUES ($1, $2, $3, $4) RETURNING name, position, inhalt, id", titel, sitzung_id, position, inhalt)
            .fetch_one(&mut **self)
            .await?)
    }

    async fn add_antrag_to_top(&mut self, antrag_id: Uuid, top_id: Uuid) -> anyhow::Result<()> {
        sqlx::query!("INSERT INTO antragstop (antrag_id, top_id) VALUES ($1, $2)", antrag_id, top_id)
            .execute(&mut **self)
            .await?;
        Ok(())
    }

    async fn anträge_by_top(&mut self, top_id: Uuid) -> anyhow::Result<Vec<Antrag>> {
        Ok(sqlx::query_as!(Antrag,
            "SELECT anträge.id, anträge.antragstext, anträge.begründung, anträge.titel FROM anträge
                  JOIN antragstop ON anträge.id = antragstop.antrag_id WHERE antragstop.top_id = $1", top_id)
            .fetch_all(&mut **self)
            .await?)
    }

    async fn tops_by_sitzung(&mut self, sitzung_id: Uuid) -> anyhow::Result<Vec<Top>> {
        Ok(sqlx::query_as!(Top, "SELECT id, name, inhalt, position FROM tops WHERE sitzung_id = $1", sitzung_id)
            .fetch_all(&mut **self)
            .await?)
    }

}