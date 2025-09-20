use chrono::{DateTime, Utc};
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

pub(super) async fn query_antragsstellende(
    conn: &mut PgConnection,
    antrag_id: Uuid,
) -> Result<Vec<Uuid>> {
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

pub(super) async fn query_attachments(
    conn: &mut PgConnection,
    antrags_id: Uuid,
) -> Result<Vec<Uuid>> {
    let result = sqlx::query_scalar!(
        r#"
            SELECT attachment_id FROM attachment_mapping
            WHERE antrags_id = $1
        "#,
        antrags_id
    )
    .fetch_all(conn)
    .await?;

    Ok(result)
}

impl AntragRepo for PgConnection {
    async fn create_antrag(
        &mut self,
        ersteller: &[Uuid],
        title: &str,
        begruendung: &str,
        antragstext: &str,
        erstellt_am: DateTime<Utc>,
    ) -> Result<Antrag> {
        let antrag = sqlx::query_as!(
            AntragData,
            r#"
                INSERT INTO antraege (titel, antragstext, begruendung, erstellt_am) 
                VALUES ($1, $2, $3, $4) 
                RETURNING *
            "#,
            title,
            antragstext,
            begruendung,
            erstellt_am
        )
        .fetch_one(&mut *self)
        .await?;

        insert_antragsstellende(&mut *self, antrag.id, ersteller).await?;

        let result = Antrag {
            data: antrag,
            ersteller: ersteller.to_vec(),
            anhaenge: vec![],
        };

        Ok(result)
    }

    async fn antraege(&mut self) -> Result<Vec<Antrag>> {
        let anträge = sqlx::query_as!(
            AntragData,
            r#"
                SELECT
                    id,
                    titel,
                    antragstext,
                    begruendung,
                    erstellt_am 
                FROM antraege
            "#
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

    async fn antrag_by_id(&mut self, id: Uuid) -> Result<Option<Antrag>> {
        let Some(data) = sqlx::query_as!(
            AntragData,
            r#"
                SELECT 
                    antraege.id,
                    antraege.titel,
                    antraege.antragstext,
                    antraege.begruendung,
                    antraege.erstellt_am
                FROM antraege
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
        let attachments = query_attachments(&mut *self, data.id).await?;

        let result = Antrag {
            data,
            ersteller: creators,
            anhaenge: attachments,
        };

        Ok(Some(result))
    }

    async fn update_antrag<'a>(
        &mut self,
        id: Uuid,
        erstellt_am: DateTime<Utc>,
        creators: Option<&'a [Uuid]>,
        title: Option<&'a str>,
        reason: Option<&'a str>,
        antragstext: Option<&'a str>,
    ) -> Result<Option<Antrag>> {
        let Some(antrag) = sqlx::query_as!(
            AntragData,
            r#"
                UPDATE antraege
                SET
                    titel = COALESCE($1, titel),
                    begruendung = COALESCE($2, begruendung),
                    antragstext = COALESCE($3, antragstext),
                    erstellt_am = $4
                WHERE id = $5
                RETURNING *
            "#,
            title,
            reason,
            antragstext,
            erstellt_am,
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

        let attachments = query_attachments(&mut *self, id).await?;

        let result = Antrag {
            data: antrag,
            ersteller: new_creators,
            anhaenge: attachments,
        };

        Ok(Some(result))
    }

    async fn delete_antrag(&mut self, id: Uuid) -> Result<Option<AntragData>> {
        let result = sqlx::query_as!(
            AntragData,
            r#"
                DELETE FROM antraege 
                WHERE id = $1
                RETURNING *
            "#,
            id
        )
        .fetch_optional(&mut *self)
        .await?;

        Ok(result)
    }

    async fn add_anhang_to_antrag(&mut self, antrags_id: Uuid, attachment_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                INSERT INTO attachment_mapping (antrags_id, attachment_id) 
                VALUES ($1, $2)
            "#,
            antrags_id,
            attachment_id
        )
        .fetch_optional(&mut *self)
        .await?;

        Ok(())
    }

    async fn delete_anhang_from_antrag(
        &mut self,
        antrags_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM attachment_mapping 
            WHERE antrags_id = $1 AND attachment_id = $2
            "#,
            antrags_id,
            attachment_id
        )
        .fetch_optional(&mut *self)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use chrono::{DateTime, Utc};
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

        let title = "Volthahn";
        let begruendung = "Volt gut";
        let antragstext = "Wir brauchen Volt";
        let erstellt_am = DateTime::UNIX_EPOCH;

        let antrag = conn
            .create_antrag(
                creators.as_slice(),
                title,
                begruendung,
                antragstext,
                erstellt_am,
            )
            .await?;

        let creator_entries = super::query_antragsstellende(&mut conn, antrag.data.id).await?;

        assert_eq!(antrag.data.titel, title);
        assert_eq!(antrag.data.antragstext, antragstext);
        assert_eq!(antrag.data.begruendung, begruendung);
        assert_eq!(antrag.data.erstellt_am, erstellt_am);
        assert_eq!(antrag.ersteller, creators);

        assert_eq!(creator_entries, creators);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons"))]
    async fn create_antrag_no_creators(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let title = "Volthahn";
        let begruendung = "Volt gut";
        let antragstext = "Wir brauchen Volt";
        let erstellt_am = DateTime::UNIX_EPOCH;

        let antrag = conn
            .create_antrag(&[], title, begruendung, antragstext, erstellt_am)
            .await?;

        assert_eq!(antrag.data.titel, title);
        assert_eq!(antrag.data.antragstext, antragstext);
        assert_eq!(antrag.data.begruendung, begruendung);

        assert!(antrag.ersteller.is_empty());

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

        let antrag = conn.antraege().await?;

        assert!(!antrag.is_empty());

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_antraege"))]
    async fn anträge(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let anträge = conn.antraege().await?;

        let creators1 = vec![
            Uuid::parse_str("5a5a134d-9345-4c36-a466-1c3bb806b240").unwrap(),
            Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap(),
        ];

        let id1 = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();
        let title1 = "Volthahn";
        let begruendung1 = "Volt gut";
        let antragstext1 = "Wir brauchen Volt";
        let created_at1 = "2021-08-01T00:00:00Z";

        let antrag1 = Antrag {
            data: AntragData {
                titel: title1.to_string(),
                id: id1,
                antragstext: antragstext1.to_string(),
                begruendung: begruendung1.to_string(),
                erstellt_am: created_at1.parse().unwrap(),
            },
            ersteller: creators1,
            anhaenge: vec![],
        };

        let creators2 = vec![Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap()];

        let id2 = Uuid::parse_str("5c51d5c0-3943-4695-844d-4c47da854fac").unwrap();
        let title2 = "blub";
        let begruendung2 = "bulabsb";
        let antragstext2 = "blub";
        let created_at2 = "2021-08-02T00:00:00Z";

        let antrag2 = Antrag {
            data: AntragData {
                titel: title2.to_string(),
                id: id2,
                antragstext: antragstext2.to_string(),
                begruendung: begruendung2.to_string(),
                erstellt_am: created_at2.parse().unwrap(),
            },
            ersteller: creators2,
            anhaenge: vec![],
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

        let title = "Volthahn";
        let begruendung = "Volt gut";
        let antragstext = "Wir brauchen Volt";
        let erstellt_am: DateTime<Utc> = "2021-08-01T00:00:00Z".parse().unwrap();

        let antrag = conn.antrag_by_id(id).await?.unwrap();

        assert_eq!(antrag.data.titel, title);
        assert_eq!(antrag.data.antragstext, antragstext);
        assert_eq!(antrag.data.begruendung, begruendung);
        assert_eq!(antrag.data.erstellt_am, erstellt_am);
        assert_eq!(antrag.ersteller, creators);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_antraege"))]
    async fn update_antrag(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let antrag_id = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();

        let old_title = "Volthahn";
        let old_begruendung = "Volt gut";

        let new_creators = vec![
            Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap(),
            Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap(),
        ];

        let new_antragstext = "aber sie schimmelt manchmal :(";
        let new_erstellt_am = DateTime::UNIX_EPOCH;

        let antrag = conn
            .update_antrag(
                antrag_id,
                new_erstellt_am,
                Some(new_creators.as_slice()),
                None,
                None,
                Some(new_antragstext),
            )
            .await?
            .unwrap();

        assert_eq!(antrag.ersteller, new_creators);
        assert_eq!(antrag.data.antragstext, new_antragstext);
        assert_eq!(antrag.data.titel, old_title);
        assert_eq!(antrag.data.begruendung, old_begruendung);
        assert_eq!(antrag.data.erstellt_am, new_erstellt_am);

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

    #[sqlx::test(fixtures("gimme_antraege", "gimme_attachments"))]
    async fn add_attachment_to_antrag(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let antrag_id = Uuid::parse_str("5c51d5c0-3943-4695-844d-4c47da854fac").unwrap();
        let attachment_id = Uuid::parse_str("9b5104a9-6a7d-468e-bbf2-f72a9086a3dc").unwrap();

        conn.add_anhang_to_antrag(antrag_id, attachment_id).await?;

        let antrag = conn.antrag_by_id(antrag_id).await?.unwrap();

        assert_eq!(antrag.anhaenge.len(), 1);
        assert_eq!(antrag.anhaenge[0], attachment_id);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_antraege", "gimme_attachments", "gimme_attachment_mappings"))]
    async fn delete_attachment_from_antrag(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let antrag_id = Uuid::parse_str("5c51d5c0-3943-4695-844d-4c47da854fac").unwrap();
        let antrag2_id = Uuid::parse_str("46148231-87b0-4486-8043-c55038178518").unwrap();

        let attachment_id = Uuid::parse_str("9b5104a9-6a7d-468e-bbf2-f72a9086a3dc").unwrap();

        conn.delete_anhang_from_antrag(antrag_id, attachment_id)
            .await?;
        conn.delete_anhang_from_antrag(antrag2_id, attachment_id)
            .await?;

        let antrag = conn.antrag_by_id(antrag_id).await?.unwrap();
        let antrag2 = conn.antrag_by_id(antrag_id).await?.unwrap();

        assert!(antrag.anhaenge.is_empty());
        assert!(antrag2.anhaenge.is_empty());

        Ok(())
    }
}
