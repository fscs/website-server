use std::future::Future;
use std::ops::{Deref, DerefMut};

use crate::domain::Result;
use sqlx::pool::PoolConnection;
use sqlx::{PgConnection, PgPool, Postgres, Transaction, postgres::PgPoolOptions};

pub mod antrag;
pub mod antrag_top_map;
pub mod attachment;
pub mod door_state;
pub mod legislative_period;
pub mod persons;
pub mod sitzungen;

#[derive(Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

pub struct DatabaseConnection {
    connection: PoolConnection<Postgres>,
}

impl DerefMut for DatabaseConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

impl Deref for DatabaseConnection {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

#[must_use]
#[derive(Debug)]
pub struct DatabaseTransaction<'a> {
    transaction: Transaction<'a, Postgres>,
}

impl DerefMut for DatabaseTransaction<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.transaction
    }
}

impl Deref for DatabaseTransaction<'_> {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

#[allow(dead_code)]
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

    pub async fn aquire(&self) -> Result<DatabaseConnection> {
        Ok(DatabaseConnection {
            connection: self.pool().acquire().await?,
        })
    }

    pub async fn start_transaction(&self) -> Result<DatabaseTransaction<'static>> {
        Ok(DatabaseTransaction {
            transaction: self.pool.begin().await?,
        })
    }

    #[allow(dead_code)]
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
