use chrono::{DateTime, Utc};
use anyhow::Result;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{Sitzung, SitzungRepo, SitzungType};

impl SitzungRepo for PgConnection {
    async fn create_sitzung(
        &mut self,
        datetime: DateTime<Utc>,
        location: &str,
        sitzung_type: SitzungType,
    ) -> Result<Sitzung> {
        let result = sqlx::query_as!(
            Sitzung,
            r#"
                INSERT INTO sitzungen (datum, location, sitzung_type) 
                VALUES ($1, $2, $3) 
                RETURNING id, datum, location, sitzung_type AS "sitzung_type!: SitzungType"
            "#,
            datetime,
            location,
            sitzung_type as SitzungType,
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn sitzungen(&mut self) -> Result<Vec<Sitzung>> {
        let result = sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType"
                FROM sitzungen
            "#
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn sitzung_by_id(&mut self, id: Uuid) -> Result<Option<Sitzung>> {
        let result = sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType"
                FROM sitzungen
                WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn first_sitzung_after(&mut self, datetime: DateTime<Utc>) -> Result<Option<Sitzung>> {
        let result = sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType"
                FROM sitzungen
                WHERE datum > $1
                ORDER BY datum ASC
                LIMIT 1
            "#,
            datetime
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn sitzungen_between(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Sitzung>> {
        let result = sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType" 
                FROM sitzungen
                WHERE datum >= $1 AND datum <= $2
            "#,
            start,
            end
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn update_sitzung<'a>(
        &mut self,
        id: Uuid,
        datetime: Option<DateTime<Utc>>,
        location: Option<&'a str>,
        sitzung_type: Option<SitzungType>,
    ) -> Result<Sitzung> {
        let result = sqlx::query_as!(
            Sitzung,
            r#"
                UPDATE sitzungen 
                SET 
                    datum = COALESCE($1, datum),
                    location = COALESCE($2, location),
                    sitzung_type = COALESCE($3, sitzung_type)
                WHERE id = $4 
                RETURNING id, datum, location, sitzung_type AS "sitzung_type!: SitzungType" 
            "#,
            datetime,
            location,
            sitzung_type as Option<SitzungType>,
            id
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn delete_sitzung(&mut self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM tops 
                WHERE sitzung_id = $1;
            "#,
            id
        )
        .execute(&mut *self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM sitzungen 
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut *self)
        .await?;

        Ok(())
    }
}
