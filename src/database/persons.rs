use anyhow::Result;
use chrono::NaiveDate;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{Person, PersonRepo, PersonRoleMapping};

impl PersonRepo for PgConnection {
    async fn create_person(&mut self, name: &str) -> Result<Person> {
        let result = sqlx::query_as!(
            Person,
            r#"
                INSERT INTO person (name)
                VALUES ($1)
                ON CONFLICT(name) DO
                UPDATE SET name = $1
                RETURNING *
            "#,
            name
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn persons(&mut self) -> Result<Vec<Person>> {
        let result = sqlx::query_as!(
            Person,
            r#"
                SELECT * 
                FROM person
            "#
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn update_person(&mut self, id: Uuid, name: &str) -> Result<Person> {
        let result = sqlx::query_as!(
            Person,
            r#"
                UPDATE person
                SET name = $1
                WHERE id = $2
                RETURNING *
            "#,
            name,
            id
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

    async fn assign_role_to_person(
        &mut self,
        person_id: Uuid,
        role: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<PersonRoleMapping> {
        let result = sqlx::query_as!(
            PersonRoleMapping,
            r#"
                INSERT INTO rollen (person_id, rolle, anfangsdatum, ablaufdatum)
                VALUES ($1, $2, $3, $4)
                RETURNING *
            "#,
            person_id,
            role,
            start,
            end
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn persons_with_role(
        &mut self,
        role: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<Person>> {
        let result = sqlx::query_as!(
            Person,
            r#"
                SELECT id,name FROM person
                JOIN public.rollen r on person.id = r.person_id
                WHERE r.rolle = $1 AND anfangsdatum <= $2 AND ablaufdatum >= $3
            "#,
            role,
            start,
            end,
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }
}
