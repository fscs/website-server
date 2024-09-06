use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::domain::{DoorState, DoorStateRepo};

impl DoorStateRepo for PgConnection {
    async fn create_door_state(
        &mut self,
        timestamp: DateTime<Utc>,
        is_open: bool,
    ) -> Result<DoorState> {
        let result = sqlx::query_as!(
            DoorState,
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

    async fn door_state_at(&mut self, time: DateTime<Utc>) -> Result<Option<DoorState>> {
        let result = sqlx::query_as!(
            DoorState,
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

    async fn door_state_between(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<DoorState>> {
        let result = sqlx::query_as!(
            DoorState,
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

#[cfg(test)]
mod test {
    use anyhow::Result;
    use chrono::DateTime;
    use sqlx::PgPool;

    use crate::domain::DoorStateRepo;

    #[sqlx::test]
    async fn create_door_state(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let datetime = DateTime::parse_from_rfc3339("2024-09-10T10:30:00+02:00").unwrap();
        let is_open = false;

        let state = conn.create_door_state(datetime.into(), is_open).await?;

        assert_eq!(state.time, datetime);
        assert_eq!(state.is_open, is_open);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_door_states"))]
    async fn door_state_at(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let datetime = DateTime::parse_from_rfc3339("2024-09-10T12:36:00+02:00").unwrap();

        let state = conn.door_state_at(datetime.into()).await?.unwrap();

        assert!(state.is_open);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_door_states"))]
    async fn door_state_between(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let start = DateTime::parse_from_rfc3339("2024-09-10T12:32:00+02:00").unwrap();
        let end = DateTime::parse_from_rfc3339("2024-09-10T17:00:00+02:00").unwrap();

        let states = conn.door_state_between(start.into(), end.into()).await?;

        assert_eq!(states.len(), 3);

        assert!(!states[0].is_open);
        assert!(states[1].is_open);
        assert!(!states[2].is_open);

        Ok(())
    }
}
