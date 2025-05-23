use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{
    legislative_periods::LegislativePeriod,
    sitzung::{Sitzung, SitzungKind, SitzungRepo, Top, TopKind},
    Result,
};

impl SitzungRepo for PgConnection {
    async fn create_sitzung(
        &mut self,
        datetime: DateTime<Utc>,
        location: &str,
        kind: SitzungKind,
        antragsfrist: DateTime<Utc>,
        legislative_period_id: Uuid,
    ) -> Result<Sitzung> {
        let record = sqlx::query!(
            r#"
                WITH inserted as (
                    INSERT INTO sitzungen (datetime, location, kind, antragsfrist, legislative_period_id)
                    VALUES ($1, $2, $3, $4, $5) 
                    RETURNING *
                ) SELECT 
                    inserted.id, 
                    datetime, 
                    location, 
                    kind AS "kind!: SitzungKind", 
                    antragsfrist, 
                    legislative_period.id as legislative_id, 
                    legislative_period.name as legislative_name
                FROM inserted 
                JOIN legislative_period
                on inserted.legislative_period_id = legislative_period.id
            "#,
            datetime,
            location,
            kind as SitzungKind,
            antragsfrist,
            legislative_period_id
        )
        .fetch_one(self)
        .await?;

        let result = Sitzung {
            id: record.id,
            datetime: record.datetime,
            location: record.location,
            kind: record.kind,
            antragsfrist: record.antragsfrist,
            legislative_period: LegislativePeriod {
                id: record.legislative_id,
                name: record.legislative_name,
            },
        };

        Ok(result)
    }

    async fn create_top(
        &mut self,
        sitzung_id: Uuid,
        name: &str,
        inhalt: &str,
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
        let records = sqlx::query!(
            r#"
                SELECT 
                    sitzungen.id, 
                    datetime, 
                    location, 
                    kind AS "kind!: SitzungKind", 
                    antragsfrist, 
                    legislative_period.id AS legislative_id, 
                    legislative_period.name AS legislative_name
                FROM sitzungen
                JOIN legislative_period 
                ON sitzungen.legislative_period_id = legislative_period.id
            "#
        )
        .fetch_all(self)
        .await?;

        let result = records
            .into_iter()
            .map(|r| Sitzung {
                id: r.id,
                datetime: r.datetime,
                location: r.location,
                kind: r.kind,
                antragsfrist: r.antragsfrist,
                legislative_period: LegislativePeriod {
                    id: r.legislative_id,
                    name: r.legislative_name,
                },
            })
            .collect();

        Ok(result)
    }

    async fn sitzung_by_id(&mut self, id: Uuid) -> Result<Option<Sitzung>> {
        let record = sqlx::query!(
            r#"
                SELECT 
                    sitzungen.id, 
                    datetime, 
                    location, 
                    kind AS "kind!: SitzungKind", 
                    antragsfrist, 
                    legislative_period.id AS legislative_id,
                    legislative_period.name AS legislative_name
                FROM sitzungen
                JOIN legislative_period
                ON sitzungen.legislative_period_id = legislative_period.id
                WHERE sitzungen.id = $1
            "#,
            id,
        )
        .fetch_optional(self)
        .await?;

        let result = record.map(|r| Sitzung {
            id: r.id,
            datetime: r.datetime,
            location: r.location,
            kind: r.kind,
            antragsfrist: r.antragsfrist,
            legislative_period: LegislativePeriod {
                id: r.legislative_id,
                name: r.legislative_name,
            },
        });

        Ok(result)
    }

    async fn sitzungen_after(
        &mut self,
        datetime: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Sitzung>> {
        let records = sqlx::query!(
            r#"
                SELECT 
                    sitzungen.id, 
                    datetime, 
                    location, 
                    kind AS "kind!: SitzungKind", 
                    antragsfrist, 
                    legislative_period.id AS legislative_id,
                    legislative_period.name AS legislative_name
                FROM sitzungen
                JOIN legislative_period
                ON sitzungen.legislative_period_id = legislative_period.id
                WHERE datetime >= $1
                ORDER BY datetime ASC
                LIMIT $2
            "#,
            datetime,
            limit,
        )
        .fetch_all(self)
        .await?;

        let result = records
            .into_iter()
            .map(|r| Sitzung {
                id: r.id,
                datetime: r.datetime,
                location: r.location,
                kind: r.kind,
                antragsfrist: r.antragsfrist,
                legislative_period: LegislativePeriod {
                    id: r.legislative_id,
                    name: r.legislative_name,
                },
            })
            .collect();

        Ok(result)
    }

    async fn sitzungen_between(
        &mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Sitzung>> {
        let records = sqlx::query!(
            r#"
                SELECT 
                    sitzungen.id, 
                    datetime, 
                    location, 
                    kind AS "kind!: SitzungKind", 
                    antragsfrist, 
                    legislative_period.id AS legislative_id,
                    legislative_period.name AS legislative_name
                FROM sitzungen
                JOIN legislative_period
                ON sitzungen.legislative_period_id = legislative_period.id
                WHERE datetime >= $1 AND datetime <= $2
                ORDER BY datetime ASC
            "#,
            start,
            end
        )
        .fetch_all(self)
        .await?;

        let result = records
            .into_iter()
            .map(|r| Sitzung {
                id: r.id,
                datetime: r.datetime,
                location: r.location,
                kind: r.kind,
                antragsfrist: r.antragsfrist,
                legislative_period: LegislativePeriod {
                    id: r.legislative_id,
                    name: r.legislative_name,
                },
            })
            .collect();

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
                ORDER BY weight ASC
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
        antragsfrist: Option<DateTime<Utc>>,
        legislative_period_id: Option<Uuid>,
    ) -> Result<Option<Sitzung>> {
        let record = sqlx::query!(
            r#"
                WITH updated AS (
                    UPDATE sitzungen 
                    SET 
                        datetime = COALESCE($1, datetime),
                        location = COALESCE($2, location),
                        kind = COALESCE($3, kind),
                        antragsfrist = COALESCE($4, antragsfrist),
                        legislative_period_id = COALESCE($5, legislative_period_id)
                    WHERE id = $6 
                    RETURNING id, datetime, location, kind, antragsfrist, legislative_period_id
                ) SELECT 
                    updated.id, 
                    datetime, 
                    location, 
                    kind AS "kind!: SitzungKind", 
                    antragsfrist, 
                    legislative_period.id as legislative_id, 
                    legislative_period.name as legislative_name
                FROM updated 
                JOIN legislative_period
                on updated.legislative_period_id = legislative_period.id
            "#,
            datetime,
            location,
            kind as Option<SitzungKind>,
            antragsfrist,
            legislative_period_id,
            id
        )
        .fetch_optional(self)
        .await?;

        let result = record.map(|r| Sitzung {
            id: r.id,
            datetime: r.datetime,
            location: r.location,
            kind: r.kind,
            antragsfrist: r.antragsfrist,
            legislative_period: LegislativePeriod {
                id: r.legislative_id,
                name: r.legislative_name,
            },
        });

        Ok(result)
    }

    async fn update_top<'a>(
        &mut self,
        id: Uuid,
        sitzung_id: Option<Uuid>,
        name: Option<&'a str>,
        inhalt: Option<&'a str>,
        kind: Option<TopKind>,
        weight: Option<i64>,
    ) -> Result<Option<Top>> {
        let result = sqlx::query_as!(
            Top,
            r#"
                UPDATE tops
                SET 
                    sitzung_id = COALESCE($2, sitzung_id),
                    name = COALESCE($3, name),
                    kind = COALESCE($4, kind),
                    inhalt = COALESCE($5, inhalt),
                    weight = COALESCE($6, weight)
                WHERE id = $1 
                RETURNING id, name, weight, inhalt, kind AS "kind!: TopKind"
            "#,
            id,
            sitzung_id,
            name,
            kind as Option<TopKind>,
            inhalt,
            weight
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn delete_sitzung(&mut self, id: Uuid) -> Result<Option<Sitzung>> {
        let record = sqlx::query!(
            r#"
                WITH deleted AS (
                    DELETE FROM sitzungen
                    WHERE id = $1
                    RETURNING id, datetime, location, kind, antragsfrist, legislative_period_id
                ) SELECT 
                    deleted.id, 
                    datetime, 
                    location, 
                    kind AS "kind!: SitzungKind", 
                    antragsfrist, 
                    legislative_period.id as legislative_id, 
                    legislative_period.name as legislative_name
                FROM deleted 
                JOIN legislative_period
                on deleted.legislative_period_id = legislative_period.id
            "#,
            id
        )
        .fetch_optional(self)
        .await?;

        let result = record.map(|r| Sitzung {
            id: r.id,
            datetime: r.datetime,
            location: r.location,
            kind: r.kind,
            antragsfrist: r.antragsfrist,
            legislative_period: LegislativePeriod {
                id: r.legislative_id,
                name: r.legislative_name,
            },
        });

        Ok(result)
    }

    async fn delete_top(&mut self, id: Uuid) -> Result<Option<Top>> {
        let result = sqlx::query_as!(
            Top,
            r#"
                DELETE FROM tops
                WHERE id = $1
                RETURNING id, name, weight, inhalt, kind AS "kind!: TopKind"
            "#,
            id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use chrono::DateTime;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::domain::sitzung::{SitzungKind, SitzungRepo, TopKind};

    #[sqlx::test(fixtures("gimme_legislative_period"))]
    async fn create_sitzung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let datetime = DateTime::parse_from_rfc3339("2024-09-10T10:30:00+02:00").unwrap();
        let location = "ein uni raum";
        let sitzung_kind = SitzungKind::VV;
        let antragsfrist = DateTime::parse_from_rfc3339("2024-09-07T10:30:00+02:00").unwrap();

        let legislative_period_id =
            Uuid::parse_str("f2b2b2b2-2b2b-2b2b-2b2b-2b2b2b2b2b2b").unwrap();

        let sitzung = conn
            .create_sitzung(
                datetime.into(),
                location,
                sitzung_kind,
                antragsfrist.into(),
                legislative_period_id,
            )
            .await?;

        assert_eq!(sitzung.datetime, datetime);
        assert_eq!(sitzung.location, location);
        assert_eq!(sitzung.kind, sitzung_kind);
        assert_eq!(sitzung.antragsfrist, antragsfrist);
        assert_eq!(sitzung.legislative_period.id, legislative_period_id);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen"))]
    async fn create_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("dfe75b8c-8c24-4a2b-84e5-d0573a8e6f00").unwrap();

        let first_title = "hallo";
        let first_top_kind = TopKind::Normal;
        let first_inhalt = "wheres da content";

        let first_top = conn
            .create_top(sitzung_id, first_title, first_inhalt, first_top_kind)
            .await?;

        let second_title = "haaaalllo";
        let second_top_kind = TopKind::Normal;
        let second_inhalt = "zweiter inhalt";

        let second_top = conn
            .create_top(sitzung_id, second_title, second_inhalt, second_top_kind)
            .await?;

        assert_eq!(first_top.name, first_title);
        assert_eq!(first_top.kind, first_top_kind);
        assert_eq!(first_top.inhalt, first_inhalt);
        assert_eq!(first_top.weight, 1);

        assert_eq!(second_top.name, second_title);
        assert_eq!(second_top.kind, second_top_kind);
        assert_eq!(second_top.inhalt, second_inhalt);
        assert_eq!(second_top.weight, 2);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen", "gimme_tops"))]
    async fn create_top_correct_weight(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("dfe75b8c-8c24-4a2b-84e5-d0573a8e6f00").unwrap();

        let title = "hallo";
        let kind = TopKind::Normal;
        let inhalt = "mein inhalt";

        let top = conn.create_top(sitzung_id, title, inhalt, kind).await?;

        assert_eq!(top.name, title);
        assert_eq!(top.kind, kind);
        assert_eq!(top.inhalt, inhalt);
        assert_eq!(top.weight, 5);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen"))]
    async fn sitzung_by_id(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("dfe75b8c-8c24-4a2b-84e5-d0573a8e6f00").unwrap();
        let datetime = DateTime::parse_from_rfc3339("2024-09-10T12:30:00+02:00").unwrap();
        let location = "ein uni raum";
        let sitzung_kind = SitzungKind::VV;

        let sitzung = conn.sitzung_by_id(id).await?.unwrap();

        assert_eq!(sitzung.datetime, datetime);
        assert_eq!(sitzung.location, location);
        assert_eq!(sitzung.kind, sitzung_kind);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen"))]
    async fn sitzungen_after(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let timestamp = DateTime::parse_from_rfc3339("2024-09-15T00:00:00+02:00").unwrap();

        let id = Uuid::parse_str("177b861d-0447-45ce-bc56-9eb68991cbda").unwrap();

        let sitzungen = conn.sitzungen_after(timestamp.into(), Some(1)).await?;

        assert_eq!(sitzungen[0].id, id);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen"))]
    async fn sitzungen_between(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let start = DateTime::parse_from_rfc3339("2024-09-17T12:30:00+02:00").unwrap();
        let end = DateTime::parse_from_rfc3339("2024-10-11T12:30:00+02:00").unwrap();

        let sitzungen = conn.sitzungen_between(start.into(), end.into()).await?;

        assert_eq!(sitzungen.len(), 3);

        assert_eq!(
            sitzungen[0].id,
            Uuid::parse_str("177b861d-0447-45ce-bc56-9eb68991cbda").unwrap()
        );
        assert_eq!(
            sitzungen[1].id,
            Uuid::parse_str("76f4a8a9-8944-4d89-b6b8-8cdbc1acedb2").unwrap()
        );
        assert_eq!(
            sitzungen[2].id,
            Uuid::parse_str("1e89dd3e-04fc-4f66-9ab3-e8e5bedcf053").unwrap()
        );

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen", "gimme_tops"))]
    async fn top_by_id(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("44e9af7f-c183-4e82-8f3c-c421cb87f506").unwrap();

        let top = conn.top_by_id(id).await?.unwrap();

        let weight = 4;
        let top_kind = TopKind::Normal;

        assert_eq!(top.id, id);
        assert_eq!(top.weight, weight);
        assert_eq!(top.kind, top_kind);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen", "gimme_tops"))]
    async fn tops_by_sitzung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("dfe75b8c-8c24-4a2b-84e5-d0573a8e6f00").unwrap();

        let tops = conn.tops_by_sitzung(sitzung_id).await?;

        assert_eq!(tops.len(), 4);

        assert_eq!(
            tops[0].id,
            Uuid::parse_str("fd6b67df-60f2-453a-9ffc-93514c5ccdb1").unwrap()
        );
        assert_eq!(
            tops[1].id,
            Uuid::parse_str("c5f7f1cf-9c40-47de-8385-9d7e9853f57f").unwrap()
        );
        assert_eq!(
            tops[2].id,
            Uuid::parse_str("44e9af7f-c183-4e82-8f3c-c421cb87f506").unwrap()
        );
        assert_eq!(
            tops[3].id,
            Uuid::parse_str("cc035514-1303-4dc8-851b-04a62b96bcba").unwrap()
        );

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen"))]
    async fn update_sitzung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("76f4a8a9-8944-4d89-b6b8-8cdbc1acedb2").unwrap();

        let new_sitzung_kind = SitzungKind::Konsti;

        let sitzung = conn
            .update_sitzung(sitzung_id, None, None, Some(new_sitzung_kind), None, None)
            .await?
            .unwrap();

        let old_datetime = DateTime::parse_from_rfc3339("2024-09-24T12:30:00+02:00").unwrap();
        let old_location = "ein uni raum";

        assert_eq!(sitzung.id, sitzung_id);
        assert_eq!(sitzung.datetime, old_datetime);
        assert_eq!(sitzung.location, old_location);
        assert_eq!(sitzung.kind, new_sitzung_kind);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen", "gimme_tops"))]
    async fn update_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("44e9af7f-c183-4e82-8f3c-c421cb87f506").unwrap();

        let new_name = "neuer name lmao";

        let top = conn
            .update_top(top_id, None, Some(new_name), None, None, None)
            .await?
            .unwrap();

        let old_top_kind = TopKind::Normal;
        let old_weight = 4;

        assert_eq!(top.name, new_name);
        assert_eq!(top.kind, old_top_kind);
        assert_eq!(top.weight, old_weight);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen"))]
    async fn delete_sitzung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("6180cdfb-3d66-447e-9051-feb904c7245f").unwrap();

        conn.delete_sitzung(sitzung_id).await?;

        let please_dont_be_a_sitzung = conn.sitzung_by_id(sitzung_id).await?;

        assert!(please_dont_be_a_sitzung.is_none());

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen", "gimme_tops"))]
    async fn delete_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let top_id = Uuid::parse_str("cc035514-1303-4dc8-851b-04a62b96bcba").unwrap();

        conn.delete_top(top_id).await?;

        let please_dont_be_a_top = conn.top_by_id(top_id).await?;

        assert!(please_dont_be_a_top.is_none());

        Ok(())
    }
}
