use anyhow::Result;
use chrono::NaiveDate;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{Abmeldung, Person, PersonRepo, PersonRoleMapping};

// TODO: validate and create roles

impl PersonRepo for PgConnection {
    async fn create_person(&mut self, name: &str) -> Result<Person> {
        let result = sqlx::query_as!(
            Person,
            r#"
                INSERT INTO person (name)
                VALUES ($1)
                RETURNING *
            "#,
            name
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn create_abmeldung(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Abmeldung> {
        todo!()
    }

    async fn persons(&mut self) -> Result<Vec<Person>> {
        let result = sqlx::query_as!(
            Person,
            r#"
                SELECT * FROM person
            "#
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn person_by_id(&mut self, id: Uuid) -> Result<Option<Person>> {
        let result = sqlx::query_as!(
            Person,
            r#"
                SELECT * FROM person
                WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn persons_with_role(
        &mut self,
        role: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<Person>> {
        todo!()
    }

    async fn abmeldungen_by_person(&mut self, person_id: Uuid) -> Result<Vec<Abmeldung>> {
        todo!()
    }

    async fn abmeldungen_at(&mut self, date: NaiveDate) -> Result<Vec<Abmeldung>> {
        todo!()
    }

    async fn assign_role_to_person(
        &mut self,
        person_id: Uuid,
        role: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<PersonRoleMapping> {
        todo!()
    }

    async fn revoke_role_from_person(
        &mut self,
        person_id: Uuid,
        role: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<()> {
        todo!()
    }

    async fn revoke_abmeldung_from_person(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<()> {
        todo!()
    }

    async fn update_person<'a>(&mut self, id: Uuid, name: Option<&'a str>) -> Result<Person> {
        let result = sqlx::query_as!(
            Person,
            r#"
                UPDATE person 
                SET 
                    name = COALESCE($2, name)
                WHERE id = $1 
                RETURNING *
            "#,
            id,
            name
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn delete_person(&mut self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM person
                WHERE id = $1
            "#,
            id
        )
        .execute(self)
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

    use crate::domain::PersonRepo;

    #[sqlx::test]
    async fn create_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let name = "deine mutter";

        let person = conn.create_person(name).await?;

        assert_eq!(person.name, name);

        Ok(())
    }

    #[sqlx::test(fixtures())]
    #[ignore]
    async fn create_abmeldung(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;
        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons"))]
    async fn person_by_id(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap();
        let name = "deine mutter";

        let person = conn.person_by_id(id).await?.unwrap();

        assert_eq!(person.name, name);

        Ok(())
    }

    #[sqlx::test(fixtures())]
    #[ignore]
    async fn persons_with_role(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;
        Ok(())
    }

    #[sqlx::test(fixtures())]
    #[ignore]
    async fn abmeldungen_by_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;
        Ok(())
    }

    #[sqlx::test(fixtures())]
    #[ignore]
    async fn abmeldungen_at(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;
        Ok(())
    }

    #[sqlx::test(fixtures())]
    #[ignore]
    async fn assign_role_to_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;
        Ok(())
    }

    #[sqlx::test(fixtures())]
    #[ignore]
    async fn revoke_role_from_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;
        Ok(())
    }

    #[sqlx::test(fixtures())]
    #[ignore]
    async fn revoke_abmeldung_from_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;
        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons"))]
    async fn update_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap();
        let new_name = "auch meine mutter";

        let person = conn.update_person(id, Some(new_name)).await?;

        assert_eq!(person.name, new_name);

        Ok(())
    }

    #[sqlx::test(fixtures())]
    #[ignore]
    async fn delete_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;
        
        let id = Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap();

        conn.delete_person(id).await?;

        let please_dont_be_a_person = conn.person_by_id(id).await?;

        assert!(please_dont_be_a_person.is_none());
        
        Ok(())
    }
}
