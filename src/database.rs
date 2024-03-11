use std::ops::Deref;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgConnection, PgPool, Postgres, Transaction};

#[derive(Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

#[must_use]
pub struct DatabaseTransaction<'a> {
    transaction: Transaction<'a, Postgres>,
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

    pub async fn transaction(&self) -> anyhow::Result<DatabaseTransaction<'static>> {
        Ok(DatabaseTransaction {
            transaction: self.pool.begin().await?,
        })
    }
}
