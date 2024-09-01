use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use std::future::Future;
use std::ops::{Deref, DerefMut};

pub mod antrag;
pub mod door_state;
pub mod persons;
pub mod sitzungen;
pub mod antrag_top_map;

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
        &mut self.transaction
    }
}

impl<'a> Deref for DatabaseTransaction<'a> {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

impl DatabaseTransaction<'_> {
    pub async fn commit(self) -> Result<()> {
        self.transaction.commit().await?;
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        self.transaction.rollback().await?;
        Ok(())
    }
}

impl DatabasePool {
    pub async fn new(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

        Ok(DatabasePool { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn start_transaction(&self) -> Result<DatabaseTransaction<'static>> {
        Ok(DatabaseTransaction {
            transaction: self.pool.begin().await?,
        })
    }

    pub async fn transaction<
        'a,
        T: 'static,
        Fut: Future<Output = Result<(T, DatabaseTransaction<'a>)>>,
        F: Fn(DatabaseTransaction<'a>) -> Fut + 'static,
    >(
        &self,
        fun: F,
    ) -> Result<T> {
        let transaction = self.start_transaction().await?;
        let (result, transaction) = fun(transaction).await?;
        transaction.commit().await?;
        Ok(result)
    }
}
