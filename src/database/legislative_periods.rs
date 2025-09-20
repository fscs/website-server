use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{
    legislatur_periode::{LegislaturPeriode, LegislaturPeriodeRepo},
    sitzung::{Sitzung, SitzungTyp},
    Result,
};

impl LegislaturPeriodeRepo for PgConnection {
    async fn create_legislatur_periode(&mut self, name: String) -> Result<LegislaturPeriode> {
        let result = sqlx::query_as!(
            LegislaturPeriode,
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

    async fn legislatur_periode_by_id(&mut self, id: Uuid) -> Result<Option<LegislaturPeriode>> {
        let result = sqlx::query_as!(
            LegislaturPeriode,
            r#"
                SELECT * FROM legislative_period WHERE id = $1 
           "#,
            id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn legislatur_perioden(&mut self) -> Result<Vec<LegislaturPeriode>> {
        let result = sqlx::query_as!(
            LegislaturPeriode,
            r#"
                SELECT * FROM legislative_period
            "#,
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn legislative_period_sitzungen(&mut self, id: Uuid) -> Result<Vec<Sitzung>> {
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
                WHERE legislative_period_id = $1
                ORDER BY datetime ASC
                "#,
            id
        )
        .fetch_all(&mut *self)
        .await?;

        let result = records
            .into_iter()
            .map(|r| Sitzung {
                id: r.id,
                datetime: r.datetime,
                ort: r.location,
                typ: r.kind,
                antragsfrist: r.antragsfrist,
                legislatur_periode: LegislaturPeriode {
                    id: r.legislative_id,
                    name: r.legislative_name,
                },
            })
            .collect();

        Ok(result)
    }

    async fn update_legislatur_periode(
        &mut self,
        id: uuid::Uuid,
        name: String,
    ) -> Result<Option<LegislaturPeriode>> {
        let result = sqlx::query_as!(
            LegislaturPeriode,
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

    async fn delete_legislatur_periode(
        &mut self,
        id: uuid::Uuid,
    ) -> Result<Option<LegislaturPeriode>> {
        let result = sqlx::query_as!(
            LegislaturPeriode,
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

    use crate::domain::legislatur_periode::LegislaturPeriodeRepo;

    #[sqlx::test]
    async fn create_legislative(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let name = "Test".to_string();

        let legislative_period = conn.create_legislatur_periode(name.clone()).await?;

        assert_eq!(legislative_period.name, name);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_legislative_period"))]
    async fn legislative_period_by_id(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = uuid::Uuid::parse_str("f2b2b2b2-2b2b-2b2b-2b2b-2b2b2b2b2b2b").unwrap();
        let name = "Test2";

        let period = conn.legislatur_periode_by_id(id).await?.unwrap();

        assert_eq!(period.name, name);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_legislative_period"))]
    async fn delete_legislative(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = uuid::Uuid::parse_str("f2b2b2b2-2b2b-2b2b-2b2b-2b2b2b2b2b2b").unwrap();

        let last_legislative_period = conn.delete_legislatur_periode(id).await?.unwrap();

        assert_eq!(last_legislative_period.id, id);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_legislative_period"))]
    async fn patch_legislative(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = uuid::Uuid::parse_str("f2b2b2b2-2b2b-2b2b-2b2b-2b2b2b2b2b2b").unwrap();

        let name = "Test new".to_string();

        let legislative_period = conn
            .update_legislatur_periode(id, name.clone())
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
