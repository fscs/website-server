use anyhow::Result;
use chrono::NaiveDate;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{Abmeldung, AbmeldungRepo};

impl AbmeldungRepo for PgConnection {
    async fn create_abmeldung(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Abmeldung> {
        let result = sqlx::query_as!(
            Abmeldung,
            r#"
                INSERT INTO abmeldungen (person_id, anfangsdatum, ablaufdatum)
                VALUES ($1, $2, $3)
                RETURNING *
            "#,
            person_id,
            start,
            end
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn abmeldungen_by_person(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<Abmeldung>> {
        let result = sqlx::query_as!(
            Abmeldung,
            r#"
                SELECT * FROM abmeldungen
                WHERE anfangsdatum <= $1 AND ablaufdatum >= $2 AND person_id = $3
            "#,
            start,
            end,
            person_id,
        )
        .fetch_all(self)
        .await?;

        return Ok(result);
    }

    async fn abmeldungen_between(
        &mut self,
        start: &NaiveDate,
        end: &NaiveDate,
    ) -> Result<Vec<Abmeldung>> {
        let result = sqlx::query_as!(
            Abmeldung,
            r#"
                SELECT * FROM abmeldungen
                WHERE anfangsdatum <= $1 AND ablaufdatum >= $2
            "#,
            start,
            end,
        )
        .fetch_all(self)
        .await?;

        return Ok(result);
    }
}
