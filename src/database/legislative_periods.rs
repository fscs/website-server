use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{
    legislative_periods::{LegislativePeriod, LegislativePeriodRepo},
    sitzung::{Sitzung, SitzungKind, SitzungWithTops},
    top_with_anträge_by_sitzung, Result,
};

impl LegislativePeriodRepo for PgConnection {
    async fn create_legislative_period(&mut self, name: String) -> Result<LegislativePeriod> {
        let result = sqlx::query_as!(
            LegislativePeriod,
            r#"
                INSERT INTO legislative_period (name)
                VALUES ($1)
                RETURNING *
            "#,
            name,
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn legislativ_period_by_id(&mut self, id: Uuid) -> Result<Option<LegislativePeriod>> {
        let result = sqlx::query_as!(
            LegislativePeriod,
            r#"
                SELECT * FROM legislative_period WHERE id = $1 
           "#,
            id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn legislative_periods(&mut self) -> Result<Vec<LegislativePeriod>> {
        let result = sqlx::query_as!(
            LegislativePeriod,
            r#"
                SELECT * FROM legislative_period
            "#,
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn legislative_period_sitzungen(&mut self, id: Uuid) -> Result<Vec<SitzungWithTops>> {
        let sitzungen = sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datetime, location, kind AS "kind!: SitzungKind", antragsfrist, legislative_period_id
                FROM sitzungen
                WHERE legislative_period_id = $1
                ORDER BY datetime ASC
                "#,
            id
        )
        .fetch_all(&mut *self)
        .await?;

        let mut sitzungen_with_tops = vec![];

        for sitzung in &sitzungen {
            let tops_with_anträge = top_with_anträge_by_sitzung(self, sitzung.id).await?;
            sitzungen_with_tops.push(SitzungWithTops {
                sitzung: sitzung.clone(),
                tops: tops_with_anträge,
            });
        }

        Ok(sitzungen_with_tops)
    }

    async fn update_legislative_period(
        &mut self,
        id: uuid::Uuid,
        name: String,
    ) -> Result<Option<LegislativePeriod>> {
        let result = sqlx::query_as!(
            LegislativePeriod,
            r#"
                UPDATE legislative_period
                SET name = $2
                WHERE id = $1
                RETURNING *
            "#,
            id,
            name
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn delete_legislative_period(
        &mut self,
        id: uuid::Uuid,
    ) -> Result<Option<LegislativePeriod>> {
        let result = sqlx::query_as!(
            LegislativePeriod,
            r#"
                DELETE FROM legislative_period
                WHERE id = $1
                RETURNING *
            "#,
            id,
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

    use crate::domain::legislative_periods::LegislativePeriodRepo;

    #[sqlx::test]
    async fn create_legislative(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let name = "Test".to_string();

        let legislative_period = conn.create_legislative_period(name.clone()).await?;

        assert_eq!(legislative_period.name, name);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_legislative_period"))]
    async fn legislative_period_by_id(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = uuid::Uuid::parse_str("f2b2b2b2-2b2b-2b2b-2b2b-2b2b2b2b2b2b").unwrap();
        let name = "Test2";

        let period = conn.legislativ_period_by_id(id).await?.unwrap();

        assert_eq!(period.name, name);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_legislative_period"))]
    async fn delete_legislative(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = uuid::Uuid::parse_str("f2b2b2b2-2b2b-2b2b-2b2b-2b2b2b2b2b2b").unwrap();

        let last_legislative_period = conn.delete_legislative_period(id).await?.unwrap();

        assert_eq!(last_legislative_period.id, id);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_legislative_period"))]
    async fn patch_legislative(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = uuid::Uuid::parse_str("f2b2b2b2-2b2b-2b2b-2b2b-2b2b2b2b2b2b").unwrap();

        let name = "Test new".to_string();

        let legislative_period = conn
            .update_legislative_period(id, name.clone())
            .await?
            .unwrap();

        assert_eq!(legislative_period.name, name);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_sitzungen"))]
    async fn get_legislatives_sitzungen(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = uuid::Uuid::parse_str("f4b3b3b3-3b3b-3b3b-3b3b-3b3b3b3b3b3b").unwrap();

        let sitzungen = conn.legislative_period_sitzungen(id).await?;

        assert_eq!(sitzungen.len(), 8);

        Ok(())
    }
}
