use anyhow::Result;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{Antrag, AntragRepo};

impl AntragRepo for PgConnection {
    async fn create_antrag(
        &mut self,
        creator: Uuid,
        title: &str,
        reason: &str,
        antragstext: &str,
    ) -> Result<Antrag> {
        let result = sqlx::query_as!(
            Antrag,
            r#"
                INSERT INTO anträge (titel, antragstext, begründung) 
                VALUES ($1, $2, $3) 
                RETURNING *
            "#,
            title,
            antragstext,
            reason
        )
        .fetch_one(&mut *self)
        .await?;

        sqlx::query!(
            r#"
                INSERT INTO antragsstellende (antrags_id, person_id) 
                VALUES ($1, $2)
            "#,
            result.id,
            creator
        )
        .execute(&mut *self)
        .await?;

        Ok(result)
    }

    async fn antrag_by_id(&mut self, id: Uuid) -> Result<Option<Antrag>> {
        let result = sqlx::query_as!(
            Antrag,
            r#"
                SELECT *
                FROM anträge
                WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn anträge_by_sitzung(&mut self, sitzung_id: Uuid) -> Result<Vec<Antrag>> {
        let result = sqlx::query_as!(
            Antrag,
            r#"
                SELECT anträge.id, anträge.antragstext, anträge.begründung, anträge.titel 
                FROM anträge
                JOIN antragstop 
                ON anträge.id = antragstop.antrag_id
                JOIN tops 
                ON antragstop.top_id = tops.id 
                WHERE tops.sitzung_id = $1
            "#,
            sitzung_id
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn anträge_by_top(&mut self, top_id: Uuid) -> Result<Vec<Antrag>> {
        let result = sqlx::query_as!(
            Antrag,
            r#"
                SELECT anträge.id, anträge.antragstext, anträge.begründung, anträge.titel 
                FROM anträge
                JOIN antragstop 
                ON anträge.id = antragstop.antrag_id 
                WHERE antragstop.top_id = $1
            "#,
            top_id
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn update_antrag<'a>(
        &mut self,
        id: Uuid,
        title: Option<&'a str>,
        reason: Option<&'a str>,
        antragstext: Option<&'a str>,
    ) -> Result<Antrag> {
        let result = sqlx::query_as!(
            Antrag,
            r#"
                UPDATE anträge 
                SET 
                    titel = COALESCE($1, titel),
                    begründung = COALESCE($2, begründung),
                    antragstext = COALESCE($3, antragstext)
                WHERE id = $4 
                RETURNING *
            "#,
            title,
            reason,
            antragstext,
            id
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn delete_antrag(&mut self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM antragstop 
                WHERE antrag_id = $1
            "#,
            id
        )
        .execute(&mut *self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM antragsstellende 
                WHERE antrags_id = $1
            "#,
            id
        )
        .execute(&mut *self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM anträge 
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut *self)
        .await?;

        Ok(())
    }
}
