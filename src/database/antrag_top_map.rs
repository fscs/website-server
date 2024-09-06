use anyhow::Result;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{
    antrag::{Antrag, AntragData},
    antrag_top_map::{AntragTopMapRepo, AntragTopMapping},
};

impl AntragTopMapRepo for PgConnection {
    async fn anträge_by_top(&mut self, top_id: Uuid) -> Result<Vec<Antrag>> {
        let records = sqlx::query!(
            r#"
                SELECT 
                    anträge.id, 
                    anträge.antragstext, 
                    anträge.begründung, 
                    anträge.titel, 
                    ARRAY_AGG(antragsstellende.person_id) AS creators
                FROM anträge
                JOIN antragstop 
                ON anträge.id = antragstop.antrag_id 
                JOIN antragsstellende
                ON anträge.id = antragsstellende.antrags_id
                WHERE antragstop.top_id = $1
                GROUP BY anträge.id
            "#,
            top_id
        )
        .fetch_all(self)
        .await?;

        let result = records
            .iter()
            .map(|r| Antrag {
                data: AntragData {
                    id: r.id,
                    antragstext: r.antragstext.clone(),
                    begründung: r.begründung.clone(),
                    titel: r.titel.clone(),
                },
                creators: r.creators.clone().unwrap_or_default(),
            })
            .collect();

        Ok(result)
    }

    async fn attach_antrag_to_top(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<Option<AntragTopMapping>> {
        let result = sqlx::query_as!(
            AntragTopMapping,
            r#"
                INSERT INTO antragstop (antrag_id, top_id) 
                VALUES ($1, $2)
                ON CONFLICT
                DO NOTHING
                RETURNING *
            "#,
            antrag_id,
            top_id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn detach_antrag_from_top(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<Option<AntragTopMapping>> {
        let result = sqlx::query_as!(
            AntragTopMapping,
            r#"
                DELETE FROM antragstop 
                WHERE antrag_id = $1 AND top_id = $2
                RETURNING *
            "#,
            antrag_id,
            top_id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::domain::AntragTopMapRepo;

    #[sqlx::test(fixtures(
        "gimme_persons",
        "gimme_sitzungen",
        "gimme_tops",
        "gimme_antraege",
        "gimme_antrag_mappings"
    ))]
    async fn anträge_by_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("fd6b67df-60f2-453a-9ffc-93514c5ccdb1").unwrap();

        let anträge = conn.anträge_by_top(top_id).await?;

        let antrag_id = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();

        assert_eq!(anträge.len(), 1);

        assert!(anträge.iter().any(|e| e.data.id == antrag_id));

        Ok(())
    }

    #[sqlx::test(fixtures(
        "gimme_persons",
        "gimme_sitzungen",
        "gimme_tops",
        "gimme_antraege",
        "gimme_antrag_mappings"
    ))]
    async fn attach_antrag_to_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("fd6b67df-60f2-453a-9ffc-93514c5ccdb1").unwrap();
        let antrag_id = Uuid::parse_str("5c51d5c0-3943-4695-844d-4c47da854fac").unwrap();

        conn.attach_antrag_to_top(antrag_id, top_id).await?;

        let anträge = conn.anträge_by_top(top_id).await?;

        assert_eq!(anträge.len(), 2);

        assert!(anträge.iter().any(|e| e.data.id == antrag_id));

        Ok(())
    }

    #[sqlx::test(fixtures(
        "gimme_persons",
        "gimme_sitzungen",
        "gimme_tops",
        "gimme_antraege",
        "gimme_antrag_mappings"
    ))]
    async fn detach_antrag_from_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("fd6b67df-60f2-453a-9ffc-93514c5ccdb1").unwrap();
        let antrag_id = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();

        conn.detach_antrag_from_top(antrag_id, top_id)
            .await?
            .unwrap();

        let anträge = conn.anträge_by_top(top_id).await?;

        assert!(anträge.is_empty());

        Ok(())
    }
}
