use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{Sitzung, SitzungKind, SitzungRepo, Top, TopKind};

impl SitzungRepo for PgConnection {
    async fn create_sitzung(
        &mut self,
        datetime: DateTime<Utc>,
        location: &str,
        kind: SitzungKind,
    ) -> Result<Sitzung> {
        let result = sqlx::query_as!(
            Sitzung,
            r#"
                INSERT INTO sitzungen (datetime, location, kind) 
                VALUES ($1, $2, $3) 
                RETURNING id, datetime, location, kind AS "kind!: SitzungKind"
            "#,
            datetime,
            location,
            kind as SitzungKind,
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn create_top<'a>(
        &mut self,
        sitzung_id: Uuid,
        name: &str,
        inhalt: Option<&'a serde_json::Value>,
        kind: TopKind,
    ) -> Result<Top> {
        let weight = sqlx::query!(
            r#"
                SELECT MAX(weight)
                FROM tops 
                WHERE sitzung_id = $1 and kind = $2
            "#,
            sitzung_id,
            kind as TopKind,
        )
        .fetch_one(&mut *self)
        .await?
        .max;

        let result = sqlx::query_as!(
            Top,
            r#"
                INSERT INTO tops (name, sitzung_id, weight, inhalt, kind)
                VALUES ($1, $2, $3, $4 ,$5) 
                RETURNING id, name, weight, inhalt, kind AS "kind!: TopKind"
            "#,
            name,
            sitzung_id,
            weight.unwrap_or(0) + 1,
            inhalt,
            kind as TopKind,
        )
        .fetch_one(&mut *self)
        .await?;

        Ok(result)
    }

    async fn sitzungen(&mut self) -> Result<Vec<Sitzung>> {
        let result = sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datetime, location, kind AS "kind!: SitzungKind"
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
                SELECT id, datetime, location, kind AS "kind!: SitzungKind"
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
                SELECT id, datetime, location, kind AS "kind!: SitzungKind"
                FROM sitzungen
                WHERE datetime >= $1
                ORDER BY datetime ASC
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
                SELECT id, datetime, location, kind AS "kind!: SitzungKind" 
                FROM sitzungen
                WHERE datetime >= $1 AND datetime <= $2
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
                SELECT id, name, weight, inhalt, kind AS "kind!: TopKind"
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
                SELECT id, name, weight, inhalt, kind AS "kind!: TopKind"
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
        kind: Option<SitzungKind>,
    ) -> Result<Option<Sitzung>> {
        let result = sqlx::query_as!(
            Sitzung,
            r#"
                UPDATE sitzungen 
                SET 
                    datetime = COALESCE($1, datetime),
                    location = COALESCE($2, location),
                    kind = COALESCE($3, kind)
                WHERE id = $4 
                RETURNING id, datetime, location, kind AS "kind!: SitzungKind" 
            "#,
            datetime,
            location,
            kind as Option<SitzungKind>,
            id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn update_top<'a>(
        &mut self,
        id: Uuid,
        sitzung_id: Option<Uuid>,
        name: Option<&'a str>,
        inhalt: Option<&'a serde_json::Value>,
        kind: Option<TopKind>,
    ) -> Result<Option<Top>> {
        let result = sqlx::query_as!(
            Top,
            r#"
                UPDATE tops 
                SET 
                    sitzung_id = COALESCE($2, sitzung_id),
                    name = COALESCE($3, name),
                    kind = COALESCE($4, kind),
                    inhalt = COALESCE($5, inhalt)
                WHERE id = $1 
                RETURNING id, name, weight, inhalt, kind AS "kind!: TopKind"
            "#,
            id,
            sitzung_id,
            name,
            kind as Option<TopKind>,
            inhalt,
        )
        .fetch_optional(self)
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

    use crate::domain::{SitzungRepo, SitzungKind, TopKind};

    #[sqlx::test]
    async fn create_sitzung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let datetime = DateTime::parse_from_rfc3339("2024-09-10T10:30:00+02:00").unwrap();
        let location = "ein uni raum";
        let sitzung_kind = SitzungKind::VV;

        let sitzung = conn
            .create_sitzung(datetime.into(), location, sitzung_kind)
            .await?;

        assert_eq!(sitzung.datetime, datetime);
        assert_eq!(sitzung.location, location);
        assert_eq!(sitzung.kind, sitzung_kind);

        Ok(())
    }

    #[sqlx::test(fixtures("create_top"))]
    async fn create_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("ba788d36-4798-408b-8dd1-102095ae2d6d").unwrap();

        let first_title = "hallo";
        let first_top_kind = TopKind::Normal;

        let first_top = conn
            .create_top(sitzung_id, first_title, None, first_top_kind)
            .await?;

        let second_title = "haaaalllo";
        let second_top_kind = TopKind::Normal;

        let second_top = conn
            .create_top(sitzung_id, second_title, None, second_top_kind)
            .await?;

        assert_eq!(first_top.name, first_title);
        assert_eq!(first_top.kind, first_top_kind);
        assert_eq!(first_top.weight, 1);

        assert_eq!(second_top.name, second_title);
        assert_eq!(second_top.kind, second_top_kind);
        assert_eq!(second_top.weight, 2);

        Ok(())
    }

    #[sqlx::test(fixtures("create_top_correct_weight"))]
    async fn create_top_correct_weight(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("ba788d36-4798-408b-8dd1-102095ae2d6d").unwrap();

        let first_title = "hallo";
        let first_top_kind = TopKind::Normal;

        let first_top = conn
            .create_top(sitzung_id, first_title, None, first_top_kind)
            .await?;

        assert_eq!(first_top.name, first_title);
        assert_eq!(first_top.kind, first_top_kind);
        assert_eq!(first_top.weight, 5);

        Ok(())
    }

    #[sqlx::test(fixtures("sitzung_by_id"))]
    async fn sitzung_by_id(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("42f7e3e2-91e4-4e60-89d9-72add0230901").unwrap();
        let datetime = DateTime::parse_from_rfc3339("2024-09-10T12:30:00+02:00").unwrap();
        let location = "ein uni raum";
        let sitzung_kind = SitzungKind::VV;

        let sitzung = conn.sitzung_by_id(id).await?.unwrap();

        assert_eq!(sitzung.datetime, datetime);
        assert_eq!(sitzung.location, location);
        assert_eq!(sitzung.kind, sitzung_kind);

        Ok(())
    }

    #[sqlx::test(fixtures("first_sitzung_after"))]
    async fn first_sitzung_after(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let timestamp = DateTime::parse_from_rfc3339("2024-09-15T00:00:00+02:00").unwrap();

        let id = Uuid::parse_str("260ce9e8-7618-4117-9a17-32211a03fae7").unwrap();

        let sitzung = conn.first_sitzung_after(timestamp.into()).await?.unwrap();

        assert_eq!(sitzung.id, id);

        Ok(())
    }

    #[sqlx::test(fixtures("sitzungen_between"))]
    async fn sitzungen_between(pool: PgPool) -> Result<()> {
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
    async fn top_by_id(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("78d38fbf-b360-41ad-be0d-ddcffdd47bb2").unwrap();

        let top = conn.top_by_id(id).await?.unwrap();

        let weight = 4;
        let top_kind = TopKind::Normal;

        assert_eq!(top.id, id);
        assert_eq!(top.weight, weight);
        assert_eq!(top.kind, top_kind);

        Ok(())
    }

    #[sqlx::test(fixtures("tops_by_sitzung"))]
    async fn tops_by_sitzung(pool: PgPool) -> Result<()> {
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
    async fn update_sitzung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("ba788d36-4798-408b-8dd1-102095ae2d6d").unwrap();

        let new_sitzung_kind = SitzungKind::Konsti;

        let sitzung = conn
            .update_sitzung(sitzung_id, None, None, Some(new_sitzung_kind))
            .await?
            .unwrap();

        let old_datetime = DateTime::parse_from_rfc3339("2024-09-10T12:30:00+02:00").unwrap();
        let old_location = "ein uni raum";

        assert_eq!(sitzung.id, sitzung_id);
        assert_eq!(sitzung.datetime, old_datetime);
        assert_eq!(sitzung.location, old_location);
        assert_eq!(sitzung.kind, new_sitzung_kind);

        Ok(())
    }

    #[sqlx::test(fixtures("update_top"))]
    async fn update_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("78d38fbf-b360-41ad-be0d-ddcffdd47bb2").unwrap();

        let new_name = "neuer name lmao";

        let top = conn
            .update_top(top_id, None, Some(new_name), None, None)
            .await?
            .unwrap();

        let old_top_kind = TopKind::Normal;
        let old_weight = 4;

        assert_eq!(top.name, new_name);
        assert_eq!(top.kind, old_top_kind);
        assert_eq!(top.weight, old_weight);

        Ok(())
    }

    #[sqlx::test(fixtures("delete_sitzung"))]
    async fn delete_sitzung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("ba788d36-4798-408b-8dd1-102095ae2d6d").unwrap();

        conn.delete_sitzung(sitzung_id).await?;

        let please_dont_be_a_sitzung = conn.sitzung_by_id(sitzung_id).await?;

        assert!(please_dont_be_a_sitzung.is_none());

        Ok(())
    }

    #[sqlx::test(fixtures("delete_top"))]
    async fn delete_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("91e12cf2-a773-4c8d-a418-8cf68478db43").unwrap();

        conn.delete_top(top_id).await?;

        let please_dont_be_a_top = conn.top_by_id(top_id).await?;

        assert!(please_dont_be_a_top.is_none());

        Ok(())
    }
}
