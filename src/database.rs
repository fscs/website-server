use std::future::Future;
use std::ops::{Deref, DerefMut};
use chrono::NaiveDateTime;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;
use crate::domain::{Antrag, AntragRepo, Sitzung, SitzungRepo};

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

impl SitzungRepo for DatabaseTransaction<'_> {

    async fn create_sitzung(&mut self, date_time: NaiveDateTime, name: &str) -> anyhow::Result<Sitzung> {
        Ok(sqlx::query_as!(Sitzung, "INSERT INTO sitzungen (datum, name) VALUES ($1, $2) RETURNING *", date_time, name)
            .fetch_one(&mut **self)
            .await?)
    }

    async fn save_sitzung(&mut self, sitzung: Sitzung) -> anyhow::Result<Sitzung> {
        todo!()
    }

    async fn find_sitzung_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Option<Sitzung>> {
        Ok(sqlx::query_as!(Sitzung,  "SELECT * FROM sitzungen WHERE id = $1", uuid).fetch_optional(&mut **self).await?)
    }

    async fn find_sitzung_after(&mut self, date_time: NaiveDateTime) -> anyhow::Result<Option<Sitzung>> {
        Ok(sqlx::query_as!(Sitzung, "SELECT * FROM sitzungen WHERE datum > $1", date_time)
            .fetch_optional(&mut **self)
            .await?)
    }
}

impl AntragRepo for DatabaseTransaction<'_> {

    async fn create_antrag(&mut self, titel: &str, antragstext: &str, begründung: &str) -> anyhow::Result<Antrag> {
        Ok(sqlx::query_as!(Antrag, "INSERT INTO anträge (titel, antragstext, begründung) VALUES ($1, $2, $3) RETURNING *", titel, antragstext, begründung)
            .fetch_one(&mut **self)
            .await?)
    }
}