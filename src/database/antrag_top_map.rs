use anyhow::Result;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{Antrag, AntragTopMapRepo, AntragTopMapping};

impl AntragTopMapRepo for PgConnection {
    async fn anträge_by_top(&mut self, top_id: Uuid) -> Result<Vec<Antrag>> {
        todo!()
        // let result = sqlx::query_as!(
        //     Antrag,
        //     r#"
        //         SELECT anträge.id, anträge.antragstext, anträge.begründung, anträge.titel 
        //         FROM anträge
        //         JOIN antragstop 
        //         ON anträge.id = antragstop.antrag_id 
        //         WHERE antragstop.top_id = $1
        //     "#,
        //     top_id
        // )
        // .fetch_all(self)
        // .await?;

        // Ok(result)
    }

    async fn attach_antrag_to_top(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<AntragTopMapping> {
        let map = sqlx::query_as!(
            AntragTopMapping,
            r#"
                INSERT INTO antragstop (antrag_id, top_id) 
                VALUES ($1, $2)
                RETURNING *
            "#,
            antrag_id,
            top_id
        );
        let result = map.fetch_one(self).await?;

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
        .execute(self)
        .await?;

        Ok(())
    }
}
