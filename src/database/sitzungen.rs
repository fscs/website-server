use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{
    legislatur_periode::LegislaturPeriode,
    sitzung::{Sitzung, SitzungRepo, SitzungTyp, Top, TopTyp},
    Result,
};

impl SitzungRepo for PgConnection {
    async fn create_sitzung(
        &mut self,
        datetime: DateTime<Utc>,
        ort: &str,
        typ: SitzungTyp,
        antragsfrist: DateTime<Utc>,
        legislatur_periode_id: Uuid,
    ) -> Result<Sitzung> {
        let record = sqlx::query!(
            r#"
                WITH inserted as (
                    INSERT INTO sitzungen (datetime, ort, typ, antragsfrist, legislatur_periode_id)
                    VALUES ($1, $2, $3, $4, $5) 
                    RETURNING *
                ) SELECT 
                    inserted.id, 
                    datetime, 
                    ort, 
                    typ AS "typ!: SitzungTyp", 
                    antragsfrist, 
                    legislatur_perioden.id as legislative_id,
                    legislatur_perioden.name as legislative_name
                FROM inserted 
                JOIN legislatur_perioden
                on inserted.legislatur_periode_id = legislatur_perioden.id
            "#,
            datetime,
            ort,
            typ as SitzungTyp,
            antragsfrist,
            legislatur_periode_id
        )
        .fetch_one(self)
        .await?;

        let result = Sitzung {
            id: record.id,
            datetime: record.datetime,
            ort: record.ort,
            typ: record.typ,
            antragsfrist: record.antragsfrist,
            legislatur_periode: LegislaturPeriode {
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
        typ: TopTyp,
    ) -> Result<Top> {
        let weight = sqlx::query!(
            r#"
                SELECT MAX(weight)
                FROM tops 
                WHERE sitzung_id = $1 and typ = $2
            "#,
            sitzung_id,
            typ as TopTyp,
        )
        .fetch_one(&mut *self)
        .await?
        .max;

        let result = sqlx::query_as!(
            Top,
            r#"
                INSERT INTO tops (name, sitzung_id, weight, inhalt, typ)
                VALUES ($1, $2, $3, $4 ,$5) 
                RETURNING id, name, weight, inhalt, typ AS "typ!: TopTyp"
            "#,
            name,
            sitzung_id,
            weight.unwrap_or(0) + 1,
            inhalt,
            typ as TopTyp,
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
                    ort, 
                    typ AS "typ!: SitzungTyp", 
                    antragsfrist, 
                    legislatur_perioden.id AS legislative_id, 
                    legislatur_perioden.name AS legislative_name
                FROM sitzungen
                JOIN legislatur_perioden 
                ON sitzungen.legislatur_periode_id = legislatur_perioden.id
            "#
        )
        .fetch_all(self)
        .await?;

        let result = records
            .into_iter()
            .map(|r| Sitzung {
                id: r.id,
                datetime: r.datetime,
                ort: r.ort,
                typ: r.typ,
                antragsfrist: r.antragsfrist,
                legislatur_periode: LegislaturPeriode {
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
                    ort, 
                    typ AS "typ!: SitzungTyp", 
                    antragsfrist, 
                    legislatur_perioden.id AS legislative_id,
                    legislatur_perioden.name AS legislative_name
                FROM sitzungen
                JOIN legislatur_perioden
                ON sitzungen.legislatur_periode_id = legislatur_perioden.id
                WHERE sitzungen.id = $1
            "#,
            id,
        )
        .fetch_optional(self)
        .await?;

        let result = record.map(|r| Sitzung {
            id: r.id,
            datetime: r.datetime,
            ort: r.ort,
            typ: r.typ,
            antragsfrist: r.antragsfrist,
            legislatur_periode: LegislaturPeriode {
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
                    ort, 
                    typ AS "typ!: SitzungTyp", 
                    antragsfrist, 
                    legislatur_perioden.id AS legislative_id,
                    legislatur_perioden.name AS legislative_name
                FROM sitzungen
                JOIN legislatur_perioden 
                ON sitzungen.legislatur_periode_id = legislatur_perioden.id
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
                ort: r.ort,
                typ: r.typ,
                antragsfrist: r.antragsfrist,
                legislatur_periode: LegislaturPeriode {
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
                    ort, 
                    typ AS "typ!: SitzungTyp", 
                    antragsfrist, 
                    legislatur_perioden.id AS legislative_id,
                    legislatur_perioden.name AS legislative_name
                FROM sitzungen
                JOIN legislatur_perioden
                ON sitzungen.legislatur_periode_id = legislatur_perioden.id
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
                ort: r.ort,
                typ: r.typ,
                antragsfrist: r.antragsfrist,
                legislatur_periode: LegislaturPeriode {
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
                SELECT id, name, weight, inhalt, typ AS "typ!: TopTyp"
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
                SELECT id, name, weight, inhalt, typ AS "typ!: TopTyp"
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
        ort: Option<&'a str>,
        typ: Option<SitzungTyp>,
        antragsfrist: Option<DateTime<Utc>>,
        legislative_period_id: Option<Uuid>,
    ) -> Result<Option<Sitzung>> {
        let record = sqlx::query!(
            r#"
                WITH updated AS (
                    UPDATE sitzungen 
                    SET 
                        datetime = COALESCE($1, datetime),
                        ort = COALESCE($2, ort),
                        typ = COALESCE($3, typ),
                        antragsfrist = COALESCE($4, antragsfrist),
                        legislatur_periode_id = COALESCE($5, legislatur_periode_id)
                    WHERE id = $6 
                    RETURNING id, datetime, ort, typ, antragsfrist, legislatur_periode_id
                ) SELECT 
                    updated.id, 
                    datetime, 
                    ort, 
                    typ AS "typ!: SitzungTyp", 
                    antragsfrist, 
                    legislatur_perioden.id as legislative_id, 
                    legislatur_perioden.name as legislative_name
                FROM updated 
                JOIN legislatur_perioden
                on updated.legislatur_periode_id = legislatur_perioden.id
            "#,
            datetime,
            ort,
            typ as Option<SitzungTyp>,
            antragsfrist,
            legislative_period_id,
            id
        )
        .fetch_optional(self)
        .await?;

        let result = record.map(|r| Sitzung {
            id: r.id,
            datetime: r.datetime,
            ort: r.ort,
            typ: r.typ,
            antragsfrist: r.antragsfrist,
            legislatur_periode: LegislaturPeriode {
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
        typ: Option<TopTyp>,
        weight: Option<i64>,
    ) -> Result<Option<Top>> {
        let result = sqlx::query_as!(
            Top,
            r#"
                UPDATE tops
                SET 
                    sitzung_id = COALESCE($2, sitzung_id),
                    name = COALESCE($3, name),
                    typ = COALESCE($4, typ),
                    inhalt = COALESCE($5, inhalt),
                    weight = COALESCE($6, weight)
                WHERE id = $1 
                RETURNING id, name, weight, inhalt, typ AS "typ!: TopTyp"
            "#,
            id,
            sitzung_id,
            name,
            typ as Option<TopTyp>,
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
                    RETURNING id, datetime, ort, typ, antragsfrist, legislatur_periode_id
                ) SELECT 
                    deleted.id, 
                    datetime, 
                    ort, 
                    typ AS "typ!: SitzungTyp", 
                    antragsfrist, 
                    legislatur_perioden.id as legislative_id, 
                    legislatur_perioden.name as legislative_name
                FROM deleted 
                JOIN legislatur_perioden 
                on deleted.legislatur_periode_id = legislatur_perioden.id
            "#,
            id
        )
        .fetch_optional(self)
        .await?;

        let result = record.map(|r| Sitzung {
            id: r.id,
            datetime: r.datetime,
            ort: r.ort,
            typ: r.typ,
            antragsfrist: r.antragsfrist,
            legislatur_periode: LegislaturPeriode {
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
                RETURNING id, name, weight, inhalt, typ AS "typ!: TopTyp"
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

    use crate::domain::sitzung::{SitzungRepo, SitzungTyp, TopTyp};

    #[sqlx::test(fixtures("gimme_legislative_period"))]
    async fn create_sitzung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let datetime = DateTime::parse_from_rfc3339("2024-09-10T10:30:00+02:00").unwrap();
        let location = "ein uni raum";
        let sitzung_kind = SitzungTyp::VV;
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
        assert_eq!(sitzung.ort, location);
        assert_eq!(sitzung.typ, sitzung_kind);
        assert_eq!(sitzung.antragsfrist, antragsfrist);
        assert_eq!(sitzung.legislatur_periode.id, legislative_period_id);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen"))]
    async fn create_top(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("dfe75b8c-8c24-4a2b-84e5-d0573a8e6f00").unwrap();

        let first_title = "hallo";
        let first_top_kind = TopTyp::Normal;
        let first_inhalt = "wheres da content";

        let first_top = conn
            .create_top(sitzung_id, first_title, first_inhalt, first_top_kind)
            .await?;

        let second_title = "haaaalllo";
        let second_top_kind = TopTyp::Normal;
        let second_inhalt = "zweiter inhalt";

        let second_top = conn
            .create_top(sitzung_id, second_title, second_inhalt, second_top_kind)
            .await?;

        assert_eq!(first_top.name, first_title);
        assert_eq!(first_top.typ, first_top_kind);
        assert_eq!(first_top.inhalt, first_inhalt);
        assert_eq!(first_top.weight, 1);

        assert_eq!(second_top.name, second_title);
        assert_eq!(second_top.typ, second_top_kind);
        assert_eq!(second_top.inhalt, second_inhalt);
        assert_eq!(second_top.weight, 2);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen", "gimme_tops"))]
    async fn create_top_correct_weight(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let sitzung_id = Uuid::parse_str("dfe75b8c-8c24-4a2b-84e5-d0573a8e6f00").unwrap();

        let title = "hallo";
        let kind = TopTyp::Normal;
        let inhalt = "mein inhalt";

        let top = conn.create_top(sitzung_id, title, inhalt, kind).await?;

        assert_eq!(top.name, title);
        assert_eq!(top.typ, kind);
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
        let sitzung_kind = SitzungTyp::VV;

        let sitzung = conn.sitzung_by_id(id).await?.unwrap();

        assert_eq!(sitzung.datetime, datetime);
        assert_eq!(sitzung.ort, location);
        assert_eq!(sitzung.typ, sitzung_kind);

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
        let top_kind = TopTyp::Normal;

        assert_eq!(top.id, id);
        assert_eq!(top.weight, weight);
        assert_eq!(top.typ, top_kind);

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

        let new_sitzung_kind = SitzungTyp::Konsti;

        let sitzung = conn
            .update_sitzung(sitzung_id, None, None, Some(new_sitzung_kind), None, None)
            .await?
            .unwrap();

        let old_datetime = DateTime::parse_from_rfc3339("2024-09-24T12:30:00+02:00").unwrap();
        let old_location = "ein uni raum";

        assert_eq!(sitzung.id, sitzung_id);
        assert_eq!(sitzung.datetime, old_datetime);
        assert_eq!(sitzung.ort, old_location);
        assert_eq!(sitzung.typ, new_sitzung_kind);

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

        let old_top_kind = TopTyp::Normal;
        let old_weight = 4;

        assert_eq!(top.name, new_name);
        assert_eq!(top.typ, old_top_kind);
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
