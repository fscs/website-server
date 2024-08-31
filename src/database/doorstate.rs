use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::domain::{DoorStateRepo, Doorstate};

impl DoorStateRepo for PgConnection {
    async fn create_doorstate(
        &mut self,
        timestamp: DateTime<Utc>,
        is_open: bool,
    ) -> Result<Doorstate> {
        let result = sqlx::query_as!(
            Doorstate,
            r#"
                    INSERT INTO doorstate (time, is_open)
                    VALUES ($1, $2)
                    RETURNING *
                "#,
            timestamp,
            is_open
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn remove_doorstate(&mut self, timestamp: DateTime<Utc>) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM doorstate 
                WHERE time = $1
            "#,
            timestamp
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn doorstate_at(&mut self, time: DateTime<Utc>) -> Result<Option<Doorstate>> {
        let result = sqlx::query_as!(
            Doorstate,
            r#"
                SELECT * FROM doorstate
                WHERE time < $1
                ORDER BY time DESC 
                LIMIT 1
            "#,
            time
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn doorstate_between(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Doorstate>> {
        let result = sqlx::query_as!(
            Doorstate,
            r#"
                SELECT * FROM doorstate
                WHERE time >= $1 AND time <= $2
            "#,
            start,
            end
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }
}
