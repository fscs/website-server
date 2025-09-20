use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{
    antrag::{Antrag, AntragData},
    antrag_top_attachment_map::{AntragTopAttachmentMap, AntragTopMapping},
    sitzung::{Top, TopArt},
    Result,
};

use super::antrag::query_antragsstellende;

pub(super) async fn query_attachments(
    conn: &mut PgConnection,
    antrag_id: Uuid,
) -> Result<Vec<Uuid>> {
    let result = sqlx::query_scalar!(
        r#"
            SELECT id
            FROM attachments
            JOIN attachment_mapping
            ON attachments.id = attachment_mapping.attachment_id
            WHERE attachment_mapping.antrags_id = $1
        "#,
        antrag_id
    )
    .fetch_all(conn)
    .await?;

    Ok(result)
}

impl AntragTopAttachmentMap for PgConnection {
    async fn antraege_by_top(&mut self, top_id: Uuid) -> Result<Vec<Antrag>> {
        let anträge = sqlx::query_as!(
            AntragData,
            r#"
                SELECT
                    anträge.id,
                    anträge.antragstext,
                    anträge.begründung,
                    anträge.titel,
                    anträge.created_at
                FROM anträge
                JOIN antragstop
                ON anträge.id = antragstop.antrag_id
                WHERE antragstop.top_id = $1
            "#,
            top_id
        )
        .fetch_all(&mut *self)
        .await?;

        let mut result = Vec::new();

        for data in anträge {
            let creators = query_antragsstellende(&mut *self, data.id).await?;
            let attachments = query_attachments(&mut *self, data.id).await?;

            result.push(Antrag {
                data: data.clone(),
                ersteller: creators,
                anhaenge: attachments,
            })
        }

        Ok(result)
    }

    async fn tops_by_antrag(
        &mut self,
        antrag_id: Uuid,
    ) -> Result<Vec<crate::domain::sitzung::Top>> {
        let result = sqlx::query_as!(
            Top,
            r#"
                SELECT
                    tops.id,
                    name,
                    weight,
                    inhalt,
                    kind AS "kind!: TopKind"
                FROM tops
                JOIN antragstop
                ON tops.id = antragstop.top_id
                WHERE antragstop.antrag_id = $1
            "#,
            antrag_id
        )
        .fetch_all(&mut *self)
        .await?;

        Ok(result)
    }

    async fn orphan_antraege(&mut self) -> Result<Vec<Antrag>> {
        let anträge = sqlx::query_as!(
            AntragData,
            r#"
                SELECT
                    anträge.id,
                    anträge.antragstext,
                    anträge.begründung,
                    anträge.titel,
                    anträge.created_at
                FROM anträge
                LEFT JOIN antragstop
                ON anträge.id = antragstop.antrag_id
                WHERE antragstop.antrag_id IS NULL
            "#,
        )
        .fetch_all(&mut *self)
        .await?;

        let mut result = Vec::new();

        for data in anträge {
            let creators = query_antragsstellende(&mut *self, data.id).await?;
            let attachments = query_attachments(&mut *self, data.id).await?;

            result.push(Antrag {
                data: data.clone(),
                ersteller: creators,
                anhaenge: attachments,
            })
        }

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

    use crate::domain::antrag_top_attachment_map::AntragTopAttachmentMap;

    #[sqlx::test(fixtures(
        "gimme_persons",
        "gimme_sitzungen",
        "gimme_tops",
        "gimme_antraege",
        "gimme_antrag_mappings",
        "gimme_attachments",
        "gimme_attachment_mappings",
    ))]
    async fn anträge_by_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("fd6b67df-60f2-453a-9ffc-93514c5ccdb1").unwrap();

        let anträge = conn.antraege_by_top(top_id).await?;

        let antrag_id = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();
        let attachment_id = Uuid::parse_str("9b5104a9-6a7d-468e-bbf2-f72a9086a3dc").unwrap(); 

        assert_eq!(anträge.len(), 1);

        assert_eq!(anträge[0].data.id, antrag_id);
        assert!(anträge[0].anhaenge.contains(&attachment_id));

        Ok(())
    }

    #[sqlx::test(fixtures(
        "gimme_persons",
        "gimme_sitzungen",
        "gimme_tops",
        "gimme_antraege",
        "gimme_antrag_mappings"
    ))]
    async fn tops_by_antrag(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let antrag_id = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();

        let top_id = Uuid::parse_str("fd6b67df-60f2-453a-9ffc-93514c5ccdb1").unwrap();

        let tops = conn.tops_by_antrag(antrag_id).await?;

        assert_eq!(tops.len(), 1);

        assert!(tops.iter().any(|e| e.id == top_id));

        Ok(())
    }

    #[sqlx::test(fixtures(
        "gimme_persons",
        "gimme_sitzungen",
        "gimme_tops",
        "gimme_antraege",
        "gimme_antrag_mappings"
    ))]
    async fn orphan_anträge(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let orphans = conn.orphan_antraege().await?;

        let antrag_id = Uuid::parse_str("5c51d5c0-3943-4695-844d-4c47da854fac").unwrap();

        assert_eq!(orphans.len(), 1);

        assert!(orphans.iter().any(|e| e.data.id == antrag_id));

        Ok(())
    }

    #[sqlx::test(fixtures(
        "gimme_persons",
        "gimme_sitzungen",
        "gimme_tops",
        "antrag_empty_creators",
        "antrag_empty_creators_map",
    ))]
    async fn anträge_by_top_empty_creators(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("fd6b67df-60f2-453a-9ffc-93514c5ccdb1").unwrap();

        let anträge = conn.antraege_by_top(top_id).await?;

        let antrag_id = Uuid::parse_str("f70917d9-8269-4a81-bb9b-785c3910f268").unwrap();

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

        let anträge = conn.antraege_by_top(top_id).await?;

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

        let anträge = conn.antraege_by_top(top_id).await?;

        assert!(anträge.is_empty());

        Ok(())
    }
}
