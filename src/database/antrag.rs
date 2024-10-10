use sqlx::{PgConnection, QueryBuilder};
use uuid::Uuid;

use crate::domain::{
    antrag::{Antrag, AntragData, AntragRepo},
    Result,
};

async fn insert_antragsstellende(
    conn: &mut PgConnection,
    antrag_id: Uuid,
    creators: &[Uuid],
) -> Result<()> {
    if creators.is_empty() {
        return Ok(());
    }

    let mut query_builder =
        QueryBuilder::new("INSERT INTO antragsstellende (antrags_id, person_id) ");

    query_builder.push_values(creators.iter(), |mut b, creator| {
        b.push_bind(antrag_id).push_bind(creator);
    });

    query_builder.build().execute(conn).await?;

    Ok(())
}

async fn query_antragsstellende(conn: &mut PgConnection, antrag_id: Uuid) -> Result<Vec<Uuid>> {
    let result = sqlx::query_scalar!(
        r#"
            SELECT person_id FROM antragsstellende
            WHERE antrags_id = $1
        "#,
        antrag_id
    )
    .fetch_all(conn)
    .await?;

    Ok(result)
}

impl AntragRepo for PgConnection {
    async fn create_antrag(
        &mut self,
        creators: &[Uuid],
        title: &str,
        reason: &str,
        antragstext: &str,
    ) -> Result<Antrag> {
        let antrag = sqlx::query_as!(
            AntragData,
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

        insert_antragsstellende(&mut *self, antrag.id, creators).await?;

        let result = Antrag {
            data: antrag,
            creators: creators.to_vec(),
        };

        Ok(result)
    }

    async fn anträge(&mut self) -> Result<Vec<Antrag>> {
        let anträge = sqlx::query_as!(
            AntragData,
            r#"
                SELECT
                    id,
                    titel,
                    antragstext,
                    begründung
                FROM anträge
            "#
        )
        .fetch_all(&mut *self)
        .await?;

        let mut result = Vec::new();

        for data in anträge {
            let creators = query_antragsstellende(&mut *self, data.id).await?;

            result.push(Antrag {
                data: data.clone(),
                creators,
            })
        }

        Ok(result)
    }

    async fn antrag_by_id(&mut self, id: Uuid) -> Result<Option<Antrag>> {
        let Some(data) = sqlx::query_as!(
            AntragData,
            r#"
                SELECT 
                    anträge.id,
                    anträge.titel,
                    anträge.antragstext,
                    anträge.begründung
                FROM anträge
                WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&mut *self)
        .await?
        else {
            return Ok(None);
        };

        let creators = query_antragsstellende(&mut *self, data.id).await?;

        let result = Antrag { data, creators };

        Ok(Some(result))
    }

    async fn update_antrag<'a>(
        &mut self,
        id: Uuid,
        creators: Option<&'a [Uuid]>,
        title: Option<&'a str>,
        reason: Option<&'a str>,
        antragstext: Option<&'a str>,
    ) -> Result<Option<Antrag>> {
        let Some(antrag) = sqlx::query_as!(
            AntragData,
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
        .fetch_optional(&mut *self)
        .await?
        else {
            return Ok(None);
        };

        let new_creators = if let Some(creators) = creators {
            sqlx::query!(
                r#"
                    DELETE FROM antragsstellende 
                    WHERE antrags_id = $1
                "#,
                id
            )
            .execute(&mut *self)
            .await?;

            insert_antragsstellende(&mut *self, id, creators).await?;

            creators.to_vec()
        } else {
            query_antragsstellende(&mut *self, id).await?
        };

        let result = Antrag {
            data: antrag,
            creators: new_creators,
        };

        Ok(Some(result))
    }

    async fn delete_antrag(&mut self, id: Uuid) -> Result<Option<AntragData>> {
        let result = sqlx::query_as!(
            AntragData,
            r#"
                DELETE FROM anträge 
                WHERE id = $1
                RETURNING *
            "#,
            id
        )
        .fetch_optional(&mut *self)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::domain::antrag::{Antrag, AntragData, AntragRepo};

    #[sqlx::test(fixtures("gimme_persons"))]
    async fn create_antrag(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let creators = vec![
            Uuid::parse_str("5a5a134d-9345-4c36-a466-1c3bb806b240").unwrap(),
            Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap(),
        ];

        let title = "Blumen für Valentin";
        let reason = "Valentin deserves them";
        let antragstext = "get them";

        let antrag = conn
            .create_antrag(creators.as_slice(), title, reason, antragstext)
            .await?;

        let creator_entries = super::query_antragsstellende(&mut conn, antrag.data.id).await?;

        assert_eq!(antrag.data.titel, title);
        assert_eq!(antrag.data.antragstext, antragstext);
        assert_eq!(antrag.data.begründung, reason);
        assert_eq!(antrag.creators, creators);

        assert_eq!(creator_entries, creators);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons"))]
    async fn create_antrag_no_creators(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let title = "Blumen für Valentin";
        let reason = "Valentin deserves them";
        let antragstext = "get them";

        let antrag = conn.create_antrag(&[], title, reason, antragstext).await?;

        assert_eq!(antrag.data.titel, title);
        assert_eq!(antrag.data.antragstext, antragstext);
        assert_eq!(antrag.data.begründung, reason);

        assert!(antrag.creators.is_empty());

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "antrag_empty_creators"))]
    async fn antrag_by_id_empty_creators(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("f70917d9-8269-4a81-bb9b-785c3910f268").unwrap();

        let antrag = conn.antrag_by_id(id).await?.unwrap();

        assert_eq!(antrag.data.id, id);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "antrag_empty_creators"))]
    async fn anträge_empty_creators(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let antrag = conn.anträge().await?;

        assert!(!antrag.is_empty());

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_antraege"))]
    async fn anträge(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let anträge = conn.anträge().await?;

        let creators1 = vec![
            Uuid::parse_str("5a5a134d-9345-4c36-a466-1c3bb806b240").unwrap(),
            Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap(),
        ];
        let title1 = "Blumen für Valentin";
        let reason1 = "Valentin deserves them";
        let antragstext1 = "get them";
        let id1 = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();

        let antrag1 = Antrag {
            data: AntragData {
                titel: title1.to_string(),
                id: id1,
                antragstext: antragstext1.to_string(),
                begründung: reason1.to_string(),
            },
            creators: creators1,
        };

        let creators2 = vec![Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap()];
        let title2 = "blub";
        let reason2 = "bulabsb";
        let antragstext2 = "blub";
        let id2 = Uuid::parse_str("5c51d5c0-3943-4695-844d-4c47da854fac").unwrap();

        let antrag2 = Antrag {
            data: AntragData {
                titel: title2.to_string(),
                id: id2,
                antragstext: antragstext2.to_string(),
                begründung: reason2.to_string(),
            },
            creators: creators2,
        };

        assert_eq!(anträge.len(), 2);

        assert!(anträge.contains(&antrag1));
        assert!(anträge.contains(&antrag2));

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_antraege"))]
    async fn antrag_by_id(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();

        let creators = vec![
            Uuid::parse_str("5a5a134d-9345-4c36-a466-1c3bb806b240").unwrap(),
            Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap(),
        ];

        let title = "Blumen für Valentin";
        let reason = "Valentin deserves them";
        let antragstext = "get them";

        let antrag = conn.antrag_by_id(id).await?.unwrap();

        assert_eq!(antrag.data.titel, title);
        assert_eq!(antrag.data.antragstext, antragstext);
        assert_eq!(antrag.data.begründung, reason);
        assert_eq!(antrag.creators, creators);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_antraege"))]
    async fn update_antrag(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let antrag_id = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();

        let old_title = "Blumen für Valentin";
        let old_reason = "Valentin deserves them";

        let new_creators = vec![
            Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap(),
            Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap(),
        ];

        let new_antragstext = "get them faster";

        let antrag = conn
            .update_antrag(
                antrag_id,
                Some(new_creators.as_slice()),
                None,
                None,
                Some(new_antragstext),
            )
            .await?
            .unwrap();

        assert_eq!(antrag.creators, new_creators);
        assert_eq!(antrag.data.antragstext, new_antragstext);
        assert_eq!(antrag.data.titel, old_title);
        assert_eq!(antrag.data.begründung, old_reason);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_antraege"))]
    async fn delete_antrag(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let antrag_id = Uuid::parse_str("5c51d5c0-3943-4695-844d-4c47da854fac").unwrap();

        conn.delete_antrag(antrag_id).await?;

        let please_dont_be_an_antrag = conn.antrag_by_id(antrag_id).await?;

        assert!(please_dont_be_an_antrag.is_none());

        Ok(())
    }
}
