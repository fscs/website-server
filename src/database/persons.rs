use anyhow::Result;
use chrono::NaiveDate;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::person::{Abmeldung, Person, PersonRepo, PersonRoleMapping};

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

    async fn create_role(&mut self, name: &str) -> Result<()> {
        sqlx::query!(
            r#"
                INSERT INTO roles (name)
                VALUES ($1)
            "#,
            name
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn create_abmeldung(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Abmeldung> {
        let result = sqlx::query_as!(
            Abmeldung,
            r#"
                WITH overlap AS (
                    DELETE FROM abmeldungen
                    WHERE
                        person_id = $1 AND
                        anfangsdatum <= $3 AND
                        ablaufdatum >= $2
                    RETURNING *
                )
                INSERT INTO abmeldungen 
                SELECT 
                    $1,
                    LEAST($2::date, MIN(anfangsdatum)) AS anfangsdatum, 
                    GREATEST($3::date, MAX(ablaufdatum)) AS ablaufdatum
                FROM overlap
                RETURNING *
            "#,
            person_id,
            start,
            end,
        )
        .fetch_one(self)
        .await?;

        Ok(result)
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

    async fn roles(&mut self) -> Result<Vec<String>> {
        let result = sqlx::query_scalar!(
            r#"
                SELECT name FROM roles
            "#
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn roles_by_person(&mut self, id: Uuid) -> Result<Vec<String>> {
        let result = sqlx::query_scalar!(
            r#"
                SELECT rolle FROM rolemapping
                WHERE person_id = $1
            "#,
            id
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

    async fn persons_with_role(&mut self, role: &str) -> Result<Vec<Person>> {
        let result = sqlx::query_as!(
            Person,
            r#"
                SELECT person.id, person.name
                FROM person
                JOIN rolemapping
                ON rolemapping.person_id = person.id
                WHERE rolemapping.rolle = $1
            "#,
            role,
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn abmeldungen_by_person(&mut self, person_id: Uuid) -> Result<Vec<Abmeldung>> {
        let result = sqlx::query_as!(
            Abmeldung,
            r#"
                SELECT * 
                FROM abmeldungen
                WHERE person_id = $1
            "#,
            person_id
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn abmeldungen_at(&mut self, date: NaiveDate) -> Result<Vec<Abmeldung>> {
        let result = sqlx::query_as!(
            Abmeldung,
            r#"
                SELECT *
                FROM abmeldungen
                WHERE anfangsdatum <= $1 AND ablaufdatum >= $1
            "#,
            date
        )
        .fetch_all(self)
        .await?;

        Ok(result)
    }

    async fn assign_role_to_person(
        &mut self,
        person_id: Uuid,
        role: &str,
    ) -> Result<Option<PersonRoleMapping>> {
        let result = sqlx::query_as!(
            PersonRoleMapping,
            r#"
                INSERT INTO rolemapping
                VALUES ($1, $2)
                ON CONFLICT
                DO NOTHING
                RETURNING *
            "#,
            person_id,
            role
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn revoke_role_from_person(
        &mut self,
        person_id: Uuid,
        role: &str,
    ) -> Result<Option<PersonRoleMapping>> {
        let result = sqlx::query_as!(
            PersonRoleMapping,
            r#"
                DELETE FROM rolemapping
                WHERE person_id = $1 AND rolle = $2
                RETURNING *
            "#,
            person_id,
            role
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn revoke_abmeldung_from_person(
        &mut self,
        person_id: Uuid,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Option<Abmeldung>> {
        let result = sqlx::query_as!(
            Abmeldung,
            r#"
                WITH overlap AS (
                    DELETE FROM abmeldungen
                    WHERE
                        person_id = $1 AND
                        anfangsdatum <= $3 AND
                        ablaufdatum >= $2
                    RETURNING *
                )
                INSERT INTO abmeldungen (person_id, anfangsdatum, ablaufdatum)
                SELECT * FROM (VALUES
                  ($1, (SELECT MIN(overlap.anfangsdatum) FROM overlap), $2),
                  ($1, $3, (SELECT MAX(overlap.ablaufdatum) FROM overlap))) AS bounds (person_id, anfangsdatum, ablaufdatum)
                WHERE
                    bounds.anfangsdatum < $2 OR
                    bounds.ablaufdatum > $3
                RETURNING *
            "#,
            person_id,
            start,
            end
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn update_person<'a>(
        &mut self,
        id: Uuid,
        name: Option<&'a str>,
    ) -> Result<Option<Person>> {
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
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn delete_person(&mut self, id: Uuid) -> Result<Option<Person>> {
        let result = sqlx::query_as!(
            Person,
            r#"
                DELETE FROM person
                WHERE id = $1
                RETURNING *
            "#,
            id
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }

    async fn delete_role(&mut self, name: &str) -> Result<Option<String>> {
        let result = sqlx::query_scalar!(
            r#"
                DELETE FROM roles
                WHERE name = $1
                RETURNING *
            "#,
            name
        )
        .fetch_optional(self)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use chrono::NaiveDate;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::domain::person::PersonRepo;

    #[sqlx::test]
    async fn create_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let name = "deine mutter";

        let person = conn.create_person(name).await?;

        assert_eq!(person.name, name);

        Ok(())
    }

    #[sqlx::test]
    async fn create_role(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let name = "Rat";

        conn.create_role(name).await?;

        let roles = conn.roles().await?;

        assert_eq!(roles.len(), 1);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn create_abmeldung_no_overlap(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 2, 5).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 2, 7).unwrap();

        let abmeldung = conn.create_abmeldung(person_id, start, end).await?;

        assert_eq!(abmeldung.person_id, person_id);
        assert_eq!(abmeldung.anfangsdatum, start);
        assert_eq!(abmeldung.ablaufdatum, end);

        let remaining = conn.abmeldungen_by_person(person_id).await?;
        assert_eq!(remaining.len(), 5);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn create_abmeldung_left_overlap(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 8, 27).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 9, 3).unwrap();

        let new_end = NaiveDate::from_ymd_opt(2024, 9, 7).unwrap();

        let abmeldung = conn.create_abmeldung(person_id, start, end).await?;

        assert_eq!(abmeldung.person_id, person_id);
        assert_eq!(abmeldung.anfangsdatum, start);
        assert_eq!(abmeldung.ablaufdatum, new_end);

        let remaining = conn.abmeldungen_by_person(person_id).await?;
        assert_eq!(remaining.len(), 4);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn create_abmeldung_right_overlap(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 9, 5).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 9, 12).unwrap();

        let new_start = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();

        let abmeldung = conn.create_abmeldung(person_id, start, end).await?;

        assert_eq!(abmeldung.person_id, person_id);
        assert_eq!(abmeldung.anfangsdatum, new_start);
        assert_eq!(abmeldung.ablaufdatum, end);

        let remaining = conn.abmeldungen_by_person(person_id).await?;
        assert_eq!(remaining.len(), 4);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn create_abmeldung_n_overlap(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 12, 3).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 24).unwrap();

        let abmeldung = conn.create_abmeldung(person_id, start, end).await?;

        assert_eq!(abmeldung.person_id, person_id);
        assert_eq!(abmeldung.anfangsdatum, start);
        assert_eq!(abmeldung.ablaufdatum, end);

        let remaining = conn.abmeldungen_by_person(person_id).await?;
        assert_eq!(remaining.len(), 2);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn create_abmeldung_n_overlap_no_left(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 12, 6).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 24).unwrap();

        let abmeldung = conn.create_abmeldung(person_id, start, end).await?;

        assert_eq!(abmeldung.person_id, person_id);
        assert_eq!(abmeldung.anfangsdatum, start);
        assert_eq!(abmeldung.ablaufdatum, end);

        let remaining = conn.abmeldungen_by_person(person_id).await?;
        assert_eq!(remaining.len(), 3);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_rollen"))]
    async fn roles_by_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap();

        let roles = conn.roles_by_person(id).await?;

        assert!(roles.contains(&"Rat".to_string()));
        assert!(roles.contains(&"Kooptiert".to_string()));

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

    #[sqlx::test(fixtures("gimme_persons", "gimme_rollen"))]
    async fn persons_with_role(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let role = "Rat";
        let person_id = Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap();

        let persons = conn.persons_with_role(role).await?;

        assert_eq!(persons.len(), 1);

        assert_eq!(persons[0].id, person_id);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn abmeldungen_by_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap();
        let abmeldungen = conn.abmeldungen_by_person(person_id).await?;

        assert_eq!(abmeldungen.len(), 2);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn abmeldungen_at(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let date = NaiveDate::from_ymd_opt(2024, 9, 5).unwrap();

        let abmeldungen = conn.abmeldungen_at(date).await?;

        assert_eq!(abmeldungen.len(), 3);

        assert_eq!(
            abmeldungen[0].person_id,
            Uuid::parse_str("51288f16-4442-4d7c-9606-3dce198b0601").unwrap()
        );
        assert_eq!(
            abmeldungen[1].person_id,
            Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap()
        );
        assert_eq!(
            abmeldungen[2].person_id,
            Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap()
        );

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_rollen"))]
    async fn assign_role_to_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("5a5a134d-9345-4c36-a466-1c3bb806b240").unwrap();
        let role = "Banana";

        let mapping = conn.assign_role_to_person(person_id, role).await?.unwrap();

        assert_eq!(mapping.person_id, person_id);
        assert_eq!(mapping.rolle, role);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_rollen"))]
    async fn revoke_role_from_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("5a5a134d-9345-4c36-a466-1c3bb806b240").unwrap();
        let role = "Kooptiert";

        conn.revoke_role_from_person(person_id, role).await?;

        let persons = conn.persons_with_role(role).await?;

        let still_has_role = persons.iter().any(|p| p.id == person_id);

        assert!(!still_has_role);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn revoke_abmeldung_from_person_no_overlap(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 2, 6).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 2, 24).unwrap();

        conn.revoke_abmeldung_from_person(person_id, start, end)
            .await?;

        let remaining_abmeldungen = conn.abmeldungen_by_person(person_id).await?;
        assert_eq!(remaining_abmeldungen.len(), 4);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn revoke_abmeldung_from_person_left_overlap(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 8, 27).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 9, 3).unwrap();

        let old_end = NaiveDate::from_ymd_opt(2024, 9, 7).unwrap();

        conn.revoke_abmeldung_from_person(person_id, start, end)
            .await?;

        let remaining_abmeldungen = conn.abmeldungen_by_person(person_id).await?;

        assert!(remaining_abmeldungen
            .iter()
            .any(|e| e.anfangsdatum == end && e.ablaufdatum == old_end));

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn revoke_abmeldung_from_person_right_overlap(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 9, 5).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 9, 13).unwrap();

        let old_start = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();

        conn.revoke_abmeldung_from_person(person_id, start, end)
            .await?;

        let remaining_abmeldungen = conn.abmeldungen_by_person(person_id).await?;

        assert!(remaining_abmeldungen
            .iter()
            .any(|e| e.anfangsdatum == old_start && e.ablaufdatum == start));

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn revoke_abmeldung_from_person_inner_overlap(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 8, 27).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 9, 8).unwrap();

        conn.revoke_abmeldung_from_person(person_id, start, end)
            .await?;

        let remaining_abmeldungen = conn.abmeldungen_by_person(person_id).await?;
        assert_eq!(remaining_abmeldungen.len(), 3);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_abmeldungen"))]
    async fn revoke_abmeldung_from_person_outer_overlap(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let person_id = Uuid::parse_str("78be7f57-8340-43e0-bba2-074da360ddf4").unwrap();
        let start = NaiveDate::from_ymd_opt(2024, 9, 3).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 9, 6).unwrap();

        conn.revoke_abmeldung_from_person(person_id, start, end)
            .await?;

        let left_start = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
        let left_end = NaiveDate::from_ymd_opt(2024, 9, 3).unwrap();

        let right_start = NaiveDate::from_ymd_opt(2024, 9, 6).unwrap();
        let right_end = NaiveDate::from_ymd_opt(2024, 9, 7).unwrap();

        let remaining_abmeldungen = conn.abmeldungen_by_person(person_id).await?;

        assert_eq!(remaining_abmeldungen.len(), 5);

        assert!(remaining_abmeldungen
            .iter()
            .any(|e| e.anfangsdatum == left_start && e.ablaufdatum == left_end));

        assert!(remaining_abmeldungen
            .iter()
            .any(|e| e.anfangsdatum == right_start && e.ablaufdatum == right_end));

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons"))]
    async fn update_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap();
        let new_name = "auch meine mutter";

        let person = conn.update_person(id, Some(new_name)).await?.unwrap();

        assert_eq!(person.name, new_name);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons"))]
    async fn delete_person(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let id = Uuid::parse_str("0f3107ac-745d-4077-8bbf-f9734cd66297").unwrap();

        conn.delete_person(id).await?;

        let please_dont_be_a_person = conn.person_by_id(id).await?;
        assert!(please_dont_be_a_person.is_none());

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_persons", "gimme_rollen"))]
    async fn delete_role(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        conn.delete_role("Banana").await?;

        let roles = conn.roles().await?;

        assert_eq!(roles.len(), 2);

        Ok(())
    }
}
