use anyhow::Result;
use uuid::Uuid;

use crate::domain::{Antrag, AntragTopMapping, Top, TopRepo};

use super::DatabaseTransaction;

impl TopRepo for DatabaseTransaction<'_> {
    async fn create_top<'a>(
        &mut self,
        title: &str,
        sitzung_id: Uuid,
        top_type: &str,
        inhalt: &serde_json::Value,
    ) -> Result<Top> {
        let weight = sqlx::query!(
            r#"
                SELECT COUNT(*) 
                FROM tops 
                WHERE sitzung_id = $1 and top_type = $2
            "#,
            sitzung_id,
            top_type
        )
        .fetch_one(&mut **self)
        .await?
        .count;

        let result = sqlx::query_as!(
            Top,
            r#"
                INSERT INTO tops (name, sitzung_id, weight, top_type, inhalt)
                VALUES ($1, $2, $3, $4 ,$5) 
                RETURNING name, weight, top_type, inhalt, id
            "#,
            title,
            sitzung_id,
            weight,
            top_type,
            inhalt
        )
        .fetch_one(&mut **self)
        .await?;

        Ok(result)
    }

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
        .fetch_one(&mut **self)
        .await?;

        sqlx::query!(
            r#"
                INSERT INTO antragsstellende (antrags_id, person_id) 
                VALUES ($1, $2)
            "#,
            result.id,
            creator
        )
        .execute(&mut **self)
        .await?;

        Ok(result)
    }

    async fn top_by_id(&mut self, id: Uuid) -> Result<Option<Top>> {
        let result = sqlx::query_as!(
            Top,
            r#"
                SELECT name, weight, top_type, inhalt, id
                FROM tops
                WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&mut **self)
        .await?;

        Ok(result)
    }

    async fn tops_by_sitzung(&mut self, sitzung_id: Uuid) -> Result<Vec<Top>> {
        let result = sqlx::query_as!(
            Top,
            r#"
                SELECT name, weight, top_type, inhalt, id
                FROM tops
                WHERE sitzung_id = $1
            "#,
            sitzung_id
        )
        .fetch_all(&mut **self)
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
        .fetch_optional(&mut **self)
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
        .fetch_all(&mut **self)
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
        .fetch_all(&mut **self)
        .await?;

        Ok(result)
    }

    async fn update_top<'a>(
        &mut self,
        id: Uuid,
        sitzung_id: Option<Uuid>,
        title: Option<&'a str>,
        top_type: Option<&'a str>,
        inhalt: Option<&'a serde_json::Value>,
    ) -> Result<Top> {
        let result = sqlx::query_as!(
            Top,
            r#"
                UPDATE tops 
                SET 
                    sitzung_id = COALESCE($1, sitzung_id),
                    name = COALESCE($2, name),
                    top_type = COALESCE($3, top_type),
                    inhalt = COALESCE($4, inhalt)
                WHERE id = $5 
                RETURNING name, inhalt, id, weight, top_type
            "#,
            sitzung_id,
            title,
            top_type,
            inhalt,
            id
        )
        .fetch_one(&mut **self)
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
        .fetch_one(&mut **self)
        .await?;

        Ok(result)
    }

    async fn attach_antrag_to_top(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<AntragTopMapping> {
        let result = sqlx::query_as!(
            AntragTopMapping,
            r#"
                INSERT INTO antragstop (antrag_id, top_id) 
                VALUES ($1, $2)
                RETURNING *
            "#,
            antrag_id,
            top_id
        )
        .fetch_one(&mut **self)
        .await?;

        Ok(result)
    }

    async fn detach_antrag_from_top(&mut self, antrag_id: Uuid, top_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM antragstop 
                WHERE antrag_id = $1 AND top_id = $2
            "#,
            antrag_id,
            top_id
        )
        .execute(&mut **self)
        .await?;

        Ok(())
    }

    async fn delete_antrag(&mut self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM antragstop 
                WHERE antrag_id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM antragsstellende 
                WHERE antrags_id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM anträge 
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;

        Ok(())
    }

    async fn delete_top(&mut self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM antragstop 
                WHERE top_id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM tops 
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;

        Ok(())
    }
}
