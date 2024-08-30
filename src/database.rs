use crate::domain::{
    Abmeldung, AbmeldungRepo, Antrag, AntragTopMapping, DoorStateRepo, Doorstate, Person,
    PersonRepo, PersonRoleMapping, Sitzung, SitzungType, Top, TopManagerRepo,
};

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

#[derive(Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

#[must_use]
#[derive(Debug)]
pub struct DatabaseTransaction<'a> {
    transaction: Transaction<'a, Postgres>,
}

impl<'a> DerefMut for DatabaseTransaction<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.transaction
    }
}

impl<'a> Deref for DatabaseTransaction<'a> {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

impl DatabaseTransaction<'_> {
    pub async fn commit(self) -> Result<()> {
        self.transaction.commit().await?;
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        self.transaction.rollback().await?;
        Ok(())
    }
}

impl DatabasePool {
    pub async fn new(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

        Ok(DatabasePool { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn start_transaction(&self) -> Result<DatabaseTransaction<'static>> {
        Ok(DatabaseTransaction {
            transaction: self.pool.begin().await?,
        })
    }

    pub async fn transaction<
        'a,
        T: 'static,
        Fut: Future<Output = Result<(T, DatabaseTransaction<'a>)>>,
        F: Fn(DatabaseTransaction<'a>) -> Fut + 'static,
    >(
        &self,
        fun: F,
    ) -> Result<T> {
        let transaction = self.start_transaction().await?;
        let (result, transaction) = fun(transaction).await?;
        transaction.commit().await?;
        Ok(result)
    }
}

impl TopManagerRepo for DatabaseTransaction<'_> {
    async fn create_sitzung(
        &mut self,
        date_time: DateTime<Utc>,
        location: &str,
        sitzung_type: SitzungType,
    ) -> Result<Sitzung> {
        Ok(sqlx::query_as!(
            Sitzung,
            r#"
                INSERT INTO sitzungen (datum, location, sitzung_type) 
                VALUES ($1, $2, $3) 
                RETURNING id, datum, location, sitzung_type AS "sitzung_type!: SitzungType"
            "#,
            date_time,
            location,
            sitzung_type as SitzungType,
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn create_person(&mut self, name: &str) -> Result<Person> {
        Ok(sqlx::query_as!(
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
        .fetch_one(&mut **self)
        .await?)
    }

    async fn create_antragssteller(&mut self, antrag_id: Uuid, person_id: Uuid) -> Result<()> {
        sqlx::query_as!(
            Antragssteller,
            r#"
                INSERT INTO antragsstellende (antrags_id, person_id) 
                VALUES ($1, $2)
            "#,
            antrag_id,
            person_id
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn save_sitzung(&mut self, sitzung: Sitzung) -> Result<Sitzung> {
        Ok(sqlx::query_as!(
            Sitzung,
            r#"
                UPDATE sitzungen 
                SET datum = $1, sitzung_type = $2, location = $3 
                WHERE id = $4 
                RETURNING id, datum, location, sitzung_type AS "sitzung_type!: SitzungType"
            "#,
            sitzung.datum,
            sitzung.sitzung_type as SitzungType,
            sitzung.location,
            sitzung.id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn find_sitzung_by_id(&mut self, uuid: Uuid) -> Result<Option<Sitzung>> {
        Ok(sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType" FROM sitzungen 
                WHERE id = $1
            "#,
            uuid
        )
        .fetch_optional(&mut **self)
        .await?)
    }

    async fn find_sitzung_after(&mut self, datetime: DateTime<Utc>) -> Result<Option<Sitzung>> {
        Ok(sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType"
                FROM sitzungen 
                WHERE datum > $1 
                ORDER BY datum ASC"#,
            datetime
        )
        .fetch_optional(&mut **self)
        .await?)
    }

    async fn get_sitzungen(&mut self) -> Result<Vec<Sitzung>> {
        Ok(sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType" FROM sitzungen
            "#
        )
        .fetch_all(&mut **self)
        .await?)
    }

    async fn create_antrag(
        &mut self,
        titel: &str,
        antragstext: &str,
        begründung: &str,
    ) -> Result<Antrag> {
        Ok(sqlx::query_as!(
            Antrag,
            r#"
                INSERT INTO anträge (titel, antragstext, begründung) 
                VALUES ($1, $2, $3) 
                RETURNING *
            "#,
            titel,
            antragstext,
            begründung
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn find_antrag_by_id(&mut self, uuid: Uuid) -> Result<Antrag> {
        Ok(sqlx::query_as!(
            Antrag,
            r#"
                SELECT * FROM anträge 
                WHERE id = $1
            "#,
            uuid
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn get_anträge(&mut self) -> Result<Vec<Antrag>> {
        Ok(sqlx::query_as!(
            Antrag,
            r#"
                SELECT * FROM anträge
            "#
        )
        .fetch_all(&mut **self)
        .await?)
    }

    async fn delete_antrag(&mut self, uuid: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM antragstop 
                WHERE antrag_id = $1
            "#,
            uuid
        )
        .execute(&mut **self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM antragsstellende 
                WHERE antrags_id = $1
            "#,
            uuid
        )
        .execute(&mut **self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM anträge 
                WHERE id = $1
            "#,
            uuid
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn anträge_by_sitzung(&mut self, sitzung_id: Uuid) -> Result<Vec<Antrag>> {
        Ok(sqlx::query_as!(
            Antrag,
            r#"
                SELECT anträge.id, anträge.antragstext, anträge.begründung, anträge.titel 
                FROM anträge
                JOIN antragstop 
                ON anträge.id = antragstop.antrag_id
                JOIN tops 
                ON antragstop.top_id = tops.id 
                WHERE tops.sitzung_id = $1
            "#,
            sitzung_id
        )
        .fetch_all(&mut **self)
        .await?)
    }

    async fn create_top(
        &mut self,
        titel: &str,
        sitzung_id: Uuid,
        top_type: &str,
        inhalt: &Option<Value>,
    ) -> Result<Top> {
        let weight = sqlx::query!(
            r#"
                SELECT COUNT(*) 
                FROM tops 
                WHERE sitzung_id = $1 and top_type = $2
            "#,
            sitzung_id,
            top_type
        )
        .fetch_one(&mut **self)
        .await?
        .count;

        Ok(sqlx::query_as!(
            Top,
            r#"
                INSERT INTO tops (name, sitzung_id, weight, top_type, inhalt)
                VALUES ($1, $2, $3, $4 ,$5) 
                RETURNING name, weight, top_type, inhalt, id
            "#,
            titel,
            sitzung_id,
            weight,
            top_type,
            *inhalt
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn add_antrag_to_top(&mut self, antrag_id: Uuid, top_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                INSERT INTO antragstop (antrag_id, top_id) 
                VALUES ($1, $2)
            "#,
            antrag_id,
            top_id
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn anträge_by_top(&mut self, top_id: Uuid) -> Result<Vec<Antrag>> {
        Ok(sqlx::query_as!(
            Antrag,
            r#"
                SELECT anträge.id, anträge.antragstext, anträge.begründung, anträge.titel 
                FROM anträge
                JOIN antragstop 
                ON anträge.id = antragstop.antrag_id 
                WHERE antragstop.top_id = $1
            "#,
            top_id
        )
        .fetch_all(&mut **self)
        .await?)
    }

    async fn tops_by_sitzung(&mut self, sitzung_id: Uuid) -> Result<Vec<Top>> {
        Ok(sqlx::query_as!(
            Top,
            r#"
                SELECT id, name, inhalt, weight, top_type 
                FROM tops 
                WHERE sitzung_id = $1 
                ORDER BY weight
            "#,
            sitzung_id
        )
        .fetch_all(&mut **self)
        .await?)
    }

    async fn get_next_sitzung(&mut self) -> Result<Option<Sitzung>> {
        let now = chrono::Utc::now();
        Ok(sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType"
                FROM sitzungen 
                WHERE datum > $1 
                ORDER BY datum ASC
            "#,
            now
        )
        .fetch_optional(&mut **self)
        .await?)
    }

    async fn get_sitzung_by_date(&mut self, datetime: DateTime<Utc>) -> Result<Option<Sitzung>> {
        Ok(sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType"
                FROM sitzungen 
                WHERE datum > $1 
                ORDER BY datum ASC
            "#,
            datetime
        )
        .fetch_optional(&mut **self)
        .await?)
    }

    async fn update_sitzung(
        &mut self,
        id: Uuid,
        datum: DateTime<Utc>,
        location: &str,
        sitzung_type: SitzungType,
    ) -> Result<Sitzung> {
        Ok(sqlx::query_as!(
            Sitzung,
            r#"
                UPDATE sitzungen 
                SET datum = $1, sitzung_type = $2, location = $3 
                WHERE id = $4 
                RETURNING id, datum, location, sitzung_type AS "sitzung_type!: SitzungType" 
            "#,
            datum,
            sitzung_type as SitzungType,
            location,
            id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_sitzung(&mut self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM tops 
                WHERE sitzung_id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM sitzungen 
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;

        Ok(())
    }

    async fn update_top(
        &mut self,
        sitzung_id: Uuid,
        id: Uuid,
        titel: &str,
        top_type: &str,
        inhalt: &Option<Value>,
    ) -> Result<Top> {
        Ok(sqlx::query_as!(
            Top,
            r#"
                UPDATE tops 
                SET name = $1, inhalt = $2, sitzung_id = $3, top_type = $4 
                WHERE id = $5 
                RETURNING name, inhalt, id, weight, top_type
            "#,
            titel,
            *inhalt,
            sitzung_id,
            top_type,
            id,
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_top(&mut self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM antragstop 
                WHERE top_id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;

        sqlx::query!(
            r#"
                DELETE FROM tops 
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;

        Ok(())
    }

    async fn create_antrag_top_mapping(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> Result<AntragTopMapping> {
        Ok(sqlx::query_as!(
            AntragTopMapping,
            r#"
                INSERT INTO antragstop (antrag_id, top_id) 
                VALUES ($1, $2) 
                RETURNING *
            "#,
            antrag_id,
            top_id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_antrag_top_mapping(&mut self, antrag_id: Uuid, top_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM antragstop 
                WHERE antrag_id = $1 AND top_id = $2
            "#,
            antrag_id,
            top_id
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn get_sitzung(&mut self, id: Uuid) -> Result<Option<Sitzung>> {
        Ok(sqlx::query_as!(
            Sitzung,
            r#"
                SELECT id, datum, location, sitzung_type AS "sitzung_type!: SitzungType" FROM sitzungen 
                WHERE id = $1
                "#,
            id
        )
        .fetch_optional(&mut **self)
        .await?)
    }
}

impl DoorStateRepo for DatabaseTransaction<'_> {
    async fn add_doorstate(&mut self, time: DateTime<Utc>, is_open: bool) -> Result<Doorstate> {
        Ok(sqlx::query_as!(
            Doorstate,
            r#"
                INSERT INTO doorstate (time, is_open) 
                VALUES ($1, $2) 
                RETURNING *
            "#,
            time,
            is_open
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn get_doorstate(&mut self, time: DateTime<Utc>) -> Result<Option<Doorstate>> {
        Ok(sqlx::query_as!(
            Doorstate,
            r#"
                SELECT * FROM doorstate 
                WHERE time < $1 
                ORDER BY time DESC LIMIT 1
            "#,
            time
        )
        .fetch_optional(&mut **self)
        .await?)
    }

    async fn get_doorstate_history(&mut self) -> Result<Option<Vec<Doorstate>>> {
        Ok(Some(
            sqlx::query_as!(
                Doorstate,
                r#"
                    SELECT * FROM doorstate
                "#
            )
            .fetch_all(&mut **self)
            .await?,
        ))
    }
}

impl PersonRepo for DatabaseTransaction<'_> {
    async fn patch_person(&mut self, id: Uuid, name: &str) -> Result<Person> {
        Ok(sqlx::query_as!(
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
        .fetch_one(&mut **self)
        .await?)
    }

    async fn add_person_role_mapping(
        &mut self,
        person_id: Uuid,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<PersonRoleMapping> {
        Ok(sqlx::query_as!(
            PersonRoleMapping,
            r#"
                INSERT INTO rollen (person_id, rolle, anfangsdatum, ablaufdatum) 
                VALUES ($1, $2, $3, $4) 
                RETURNING *
            "#,
            person_id,
            rolle,
            anfangsdatum,
            ablaufdatum
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn update_person_role_mapping(
        &mut self,
        person_id: Uuid,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<PersonRoleMapping> {
        Ok(sqlx::query_as!(
            PersonRoleMapping,
            r#"
                UPDATE rollen 
                SET rolle = $1, anfangsdatum = $2, ablaufdatum = $3 
                WHERE person_id = $4 
                RETURNING *
            "#,
            rolle,
            anfangsdatum,
            ablaufdatum,
            person_id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_person_role_mapping(&mut self, person_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM rollen 
                WHERE person_id = $1
            "#,
            person_id,
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn create_person(&mut self, name: &str) -> Result<Person> {
        Ok(sqlx::query_as!(
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
        .fetch_one(&mut **self)
        .await?)
    }

    async fn get_persons(&mut self) -> Result<Vec<Person>> {
        Ok(sqlx::query_as!(
            Person,
            r#"
                SELECT * FROM person
            "#
        )
        .fetch_all(&mut **self)
        .await?)
    }

    async fn get_person_by_role(
        &mut self,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<Vec<Person>> {
        Ok(sqlx::query_as!(
            Person,
            r#"
                SELECT id,name FROM person
                JOIN public.rollen r on person.id = r.person_id
                WHERE r.rolle = $1 AND anfangsdatum <= $2 AND ablaufdatum >= $3
            "#,
            rolle,
            anfangsdatum,
            ablaufdatum
        )
        .fetch_all(&mut **self)
        .await?)
    }

    async fn update_person(
        &mut self,
        person_id: Uuid,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<PersonRoleMapping> {
        Ok(sqlx::query_as!(
            PersonRoleMapping,
            r#"
                UPDATE rollen 
                SET rolle = $1, anfangsdatum = $2, ablaufdatum = $3 
                WHERE person_id = $4 
                RETURNING *
            "#,
            rolle,
            anfangsdatum,
            ablaufdatum,
            person_id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_person(&mut self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM person 
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }
}

impl AbmeldungRepo for DatabaseTransaction<'_> {
    async fn add_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<Abmeldung> {
        Ok(sqlx::query_as!(
            Abmeldung,
            r#"
                INSERT INTO abmeldungen (person_id, anfangsdatum, ablaufdatum) 
                VALUES ($1, $2, $3) 
                RETURNING *
            "#,
            person_id,
            anfangsdatum,
            ablaufdatum
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn get_abmeldungen(&mut self) -> Result<Vec<Abmeldung>> {
        Ok(sqlx::query_as!(
            Abmeldung,
            r#"
                SELECT * FROM abmeldungen
            "#
        )
        .fetch_all(&mut **self)
        .await?)
    }

    async fn update_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<Abmeldung> {
        Ok(sqlx::query_as!(
            Abmeldung,
            r#"
                UPDATE abmeldungen 
                SET anfangsdatum = $1, ablaufdatum = $2 
                WHERE person_id = $3 
                RETURNING *
            "#,
            anfangsdatum,
            ablaufdatum,
            person_id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM abmeldungen 
                WHERE person_id = $1 AND anfangsdatum = $2 AND ablaufdatum = $3
            "#,
            person_id,
            anfangsdatum,
            ablaufdatum
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn get_abmeldungen_between(
        &mut self,
        start: &NaiveDate,
        end: &NaiveDate,
    ) -> Result<Vec<Abmeldung>> {
        let result = sqlx::query_as!(
            Abmeldung,
            r#"
                SELECT * FROM abmeldungen 
                WHERE anfangsdatum <= $1 AND ablaufdatum >= $2
            "#,
            start,
            end,
        )
        .fetch_all(&mut **self)
        .await?;

        return Ok(result);
    }
}
