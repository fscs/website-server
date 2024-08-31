use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{AntragTopMapping, Sitzung, SitzungRepo, SitzungType, Top};

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

    async fn create_top<'a>(
        &mut self,
        sitzung_id: Uuid,
        title: &str,
        top_type: &str,
        inhalt: Option<&'a serde_json::Value>,
    ) -> Result<Top> {
        let weight = sqlx::query!(
            r#"
                SELECT MAX(weight)
                FROM tops 
                WHERE sitzung_id = $1 and top_type = $2
            "#,
            sitzung_id,
            top_type
        )
        .fetch_one(&mut *self)
        .await?
        .max;

        let result = sqlx::query_as!(
            Top,
            r#"
                INSERT INTO tops (name, sitzung_id, weight, top_type, inhalt)
                VALUES ($1, $2, $3, $4 ,$5) 
                RETURNING name, weight, top_type, inhalt, id
            "#,
            title,
            sitzung_id,
            weight.unwrap_or(0) + 1,
            top_type,
            inhalt
        )
        .fetch_one(&mut *self)
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
                WHERE datum >= $1
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
        .fetch_optional(self)
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
        .fetch_one(self)
        .await?;

        Ok(result)
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

    async fn delete_top(&mut self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM antragstop 
                WHERE top_id = $1
            "#,
            id
        )
        .execute(&mut *self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM tops 
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut *self)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use chrono::DateTime;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::domain::SitzungType;

    use super::SitzungRepo;

    #[sqlx::test]
    async fn create_sitzung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let datetime = DateTime::parse_from_rfc3339("2024-09-10T10:30:00+02:00").unwrap();
        let location = "ein uni raum";
        let sitzung_type = SitzungType::VV;

        let sitzung = conn
            .create_sitzung(datetime.into(), location, sitzung_type)
            .await?;

        assert_eq!(sitzung.datum, datetime);
        assert_eq!(sitzung.location, location);
        assert_eq!(sitzung.sitzung_type, sitzung_type);

        Ok(())
    }

    #[sqlx::test(fixtures("create_top"))]
    async fn create_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("ba788d36-4798-408b-8dd1-102095ae2d6d").unwrap();

        let first_title = "hallo";
        let first_top_type = "normal";

        let first_top = conn
            .create_top(sitzung_id, first_title, first_top_type, None)
            .await?;

        let second_title = "haaaalllo";
        let second_top_type = "normal";

        let second_top = conn
            .create_top(sitzung_id, second_title, second_top_type, None)
            .await?;

        assert_eq!(first_top.name, first_title);
        assert_eq!(first_top.top_type, first_top_type);
        assert_eq!(first_top.weight, 1);

        assert_eq!(second_top.name, second_title);
        assert_eq!(second_top.top_type, second_top_type);
        assert_eq!(second_top.weight, 2);

        Ok(())
    }

    #[sqlx::test(fixtures("create_top_correct_weight"))]
    async fn create_top_correct_weight(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("ba788d36-4798-408b-8dd1-102095ae2d6d").unwrap();

        let first_title = "hallo";
        let first_top_type = "normal";

        let first_top = conn
            .create_top(sitzung_id, first_title, first_top_type, None)
            .await?;

        assert_eq!(first_top.name, first_title);
        assert_eq!(first_top.top_type, first_top_type);
        assert_eq!(first_top.weight, 5);

        Ok(())
    }

    #[sqlx::test(fixtures("sitzung_by_id"))]
    async fn sitzung_by_id(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("42f7e3e2-91e4-4e60-89d9-72add0230901").unwrap();
        let datetime = DateTime::parse_from_rfc3339("2024-09-10T12:30:00+02:00").unwrap();
        let location = "ein uni raum";
        let sitzung_type = SitzungType::VV;

        let sitzung = conn.sitzung_by_id(id).await?.unwrap();

        assert_eq!(sitzung.datum, datetime);
        assert_eq!(sitzung.location, location);
        assert_eq!(sitzung.sitzung_type, sitzung_type);

        Ok(())
    }

    #[sqlx::test(fixtures("first_sitzung_after"))]
    async fn first_sitzung_after(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;

        let timestamp = DateTime::parse_from_rfc3339("2024-09-15T00:00:00+02:00").unwrap();

        let id = Uuid::parse_str("260ce9e8-7618-4117-9a17-32211a03fae7").unwrap();

        let sitzung = conn.first_sitzung_after(timestamp.into()).await?.unwrap();

        assert_eq!(sitzung.id, id);

        Ok(())
    }

    #[sqlx::test(fixtures("sitzungen_between"))]
    async fn sitzungen_between(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;

        let start = DateTime::parse_from_rfc3339("2024-09-17T12:30:00+02:00").unwrap();
        let end = DateTime::parse_from_rfc3339("2024-10-11T12:30:00+02:00").unwrap();

        let sitzungen = conn.sitzungen_between(start.into(), end.into()).await?;

        assert_eq!(sitzungen.len(), 3);

        assert_eq!(
            sitzungen[0].id,
            Uuid::parse_str("51789e78-c2f6-4c67-a271-d05d95de9cab")?
        );
        assert_eq!(
            sitzungen[1].id,
            Uuid::parse_str("c92e9e19-13bd-4243-b520-55bea77fae8b")?
        );
        assert_eq!(
            sitzungen[2].id,
            Uuid::parse_str("d8398880-8598-4080-bf5b-9d063295024f")?
        );

        Ok(())
    }

    #[sqlx::test(fixtures("top_by_id"))]
    async fn top_by_id(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("78d38fbf-b360-41ad-be0d-ddcffdd47bb2").unwrap();

        let top = conn.top_by_id(id).await?.unwrap();

        let weight = 4;
        let top_type = "normal";

        assert_eq!(top.id, id);
        assert_eq!(top.weight, weight);
        assert_eq!(top.top_type, top_type);

        Ok(())
    }

    #[sqlx::test(fixtures("tops_by_sitzung"))]
    async fn tops_by_sitzung(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("ba788d36-4798-408b-8dd1-102095ae2d6d").unwrap();

        let tops = conn.tops_by_sitzung(sitzung_id).await?;

        assert_eq!(tops.len(), 2);

        assert_eq!(
            tops[0].id,
            Uuid::parse_str("78d38fbf-b360-41ad-be0d-ddcffdd47bb2")?
        );
        assert_eq!(
            tops[1].id,
            Uuid::parse_str("9cce6322-029a-498e-8385-c1f9644077a5")?
        );

        Ok(())
    }

    #[sqlx::test(fixtures("update_sitzung"))]
    async fn update_sitzung(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("ba788d36-4798-408b-8dd1-102095ae2d6d").unwrap();

        let new_sitzung_type = SitzungType::Konsti;

        let sitzung = conn
            .update_sitzung(sitzung_id, None, None, Some(new_sitzung_type))
            .await?;

        let old_datetime = DateTime::parse_from_rfc3339("2024-09-10T12:30:00+02:00").unwrap();
        let old_location = "ein uni raum";

        assert_eq!(sitzung.id, sitzung_id);
        assert_eq!(sitzung.datum, old_datetime);
        assert_eq!(sitzung.location, old_location);
        assert_eq!(sitzung.sitzung_type, new_sitzung_type);

        Ok(())
    }

    #[sqlx::test(fixtures("update_top"))]
    async fn update_top(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("78d38fbf-b360-41ad-be0d-ddcffdd47bb2").unwrap();

        let new_name = "neuer name lmao";

        let top = conn
            .update_top(top_id, None, Some(new_name), None, None)
            .await?;

        let old_top_type = "normal";
        let old_weight = 4;

        assert_eq!(top.name, new_name);
        assert_eq!(top.top_type, old_top_type);
        assert_eq!(top.weight, old_weight);

        Ok(())
    }

    #[sqlx::test(fixtures("attach_antrag_to_top"))]
    async fn attach_antrag_to_top(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("78d38fbf-b360-41ad-be0d-ddcffdd47bb2").unwrap();
        let antrag_id = Uuid::parse_str("641d6bbe-990c-4ece-9e38-dd3cd0d77460").unwrap();

        let mapping = conn.attach_antrag_to_top(antrag_id, top_id).await?;

        assert_eq!(mapping.top_id, top_id);
        assert_eq!(mapping.antrag_id, antrag_id);

        Ok(())
    }

    #[sqlx::test(fixtures("detach_antrag_from_top"))]
    async fn detach_antrag_from_top(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("78d38fbf-b360-41ad-be0d-ddcffdd47bb2").unwrap();
        let antrag_id = Uuid::parse_str("641d6bbe-990c-4ece-9e38-dd3cd0d77460").unwrap();

        conn.detach_antrag_from_top(antrag_id, top_id).await?;

        Ok(())
    }
}
