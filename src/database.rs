use crate::domain::{
    Abmeldung, AbmeldungRepo, Antrag, AntragTopMapping, DoorStateRepo, Doorstate, Person,
    PersonRepo, PersonRoleMapping, Sitzung, Top, TopManagerRepo,
};

use chrono::{NaiveDate, NaiveDateTime};
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
    pub async fn commit(self) -> anyhow::Result<()> {
        self.transaction.commit().await?;
        Ok(())
    }

    pub async fn rollback(self) -> anyhow::Result<()> {
        self.transaction.rollback().await?;
        Ok(())
    }
}

impl DatabasePool {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

        Ok(DatabasePool { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn start_transaction(&self) -> anyhow::Result<DatabaseTransaction<'static>> {
        Ok(DatabaseTransaction {
            transaction: self.pool.begin().await?,
        })
    }

    pub async fn transaction<
        'a,
        T: 'static,
        Fut: Future<Output = anyhow::Result<(T, DatabaseTransaction<'a>)>>,
        F: Fn(DatabaseTransaction<'a>) -> Fut + 'static,
    >(
        &self,
        fun: F,
    ) -> anyhow::Result<T> {
        let transaction = self.start_transaction().await?;
        let (result, transaction) = fun(transaction).await?;
        transaction.commit().await?;
        Ok(result)
    }
}

impl TopManagerRepo for DatabaseTransaction<'_> {
    async fn create_sitzung(
        &mut self,
        date_time: NaiveDateTime,
        name: &str,
        location: &str,
    ) -> anyhow::Result<Sitzung> {
        Ok(sqlx::query_as!(
            Sitzung,
            "INSERT INTO sitzungen (datum, name, location) VALUES ($1, $2, $3) RETURNING *",
            date_time,
            name,
            location
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn create_person(&mut self, name: &str) -> anyhow::Result<Person> {
        Ok(sqlx::query_as!(
            Person,
            "INSERT INTO person (name) VALUES ($1) ON CONFLICT(name) DO UPDATE SET name = $1 RETURNING *",
            name
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn create_antragssteller(
        &mut self,
        antrag_id: Uuid,
        person_id: Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query_as!(
            Antragssteller,
            "INSERT INTO antragsstellende (antrags_id, person_id) VALUES ($1, $2)",
            antrag_id,
            person_id
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn save_sitzung(&mut self, sitzung: Sitzung) -> anyhow::Result<Sitzung> {
        Ok(sqlx::query_as!(
            Sitzung,
            "UPDATE sitzungen SET datum = $1, name = $2, location = $3 WHERE id = $4 RETURNING *",
            sitzung.datum,
            sitzung.name,
            sitzung.location,
            sitzung.id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn find_sitzung_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Option<Sitzung>> {
        Ok(
            sqlx::query_as!(Sitzung, "SELECT * FROM sitzungen WHERE id = $1", uuid)
                .fetch_optional(&mut **self)
                .await?,
        )
    }

    async fn find_sitzung_after(
        &mut self,
        date_time: NaiveDateTime,
    ) -> anyhow::Result<Option<Sitzung>> {
        let now = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Berlin);

        let offset_int = now.time() - chrono::Utc::now().time();
        Ok(sqlx::query_as!(
            Sitzung,
            "SELECT * FROM sitzungen WHERE datum > $1 ORDER BY datum ASC",
            date_time + offset_int
        )
        .fetch_optional(&mut **self)
        .await?)
    }

    async fn get_sitzungen(&mut self) -> anyhow::Result<Vec<Sitzung>> {
        Ok(sqlx::query_as!(Sitzung, "SELECT * FROM sitzungen")
            .fetch_all(&mut **self)
            .await?)
    }

    async fn create_antrag(
        &mut self,
        titel: &str,
        antragstext: &str,
        begründung: &str,
    ) -> anyhow::Result<Antrag> {
        Ok(sqlx::query_as!(
            Antrag,
            "INSERT INTO anträge (titel, antragstext, begründung) VALUES ($1, $2, $3) RETURNING *",
            titel,
            antragstext,
            begründung
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn find_antrag_by_id(&mut self, uuid: Uuid) -> anyhow::Result<Antrag> {
        Ok(
            sqlx::query_as!(Antrag, "SELECT * FROM anträge WHERE id = $1", uuid)
                .fetch_one(&mut **self)
                .await?,
        )
    }

    async fn get_anträge(&mut self) -> anyhow::Result<Vec<Antrag>> {
        Ok(sqlx::query_as!(Antrag, "SELECT * FROM anträge")
            .fetch_all(&mut **self)
            .await?)
    }

    async fn delete_antrag(&mut self, uuid: Uuid) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM antragstop WHERE antrag_id = $1", uuid)
            .execute(&mut **self)
            .await?;

        sqlx::query!("DELETE FROM antragsstellende WHERE antrags_id = $1", uuid)
            .execute(&mut **self)
            .await?;

        sqlx::query!("DELETE FROM anträge WHERE id = $1", uuid)
            .execute(&mut **self)
            .await?;
        Ok(())
    }

    async fn anträge_by_sitzung(&mut self, sitzung_id: Uuid) -> anyhow::Result<Vec<Antrag>> {
        Ok(sqlx::query_as!(
            Antrag,
            "SELECT anträge.id, anträge.antragstext, anträge.begründung, anträge.titel FROM anträge
                  JOIN antragstop ON anträge.id = antragstop.antrag_id
                  JOIN tops ON antragstop.top_id = tops.id WHERE tops.sitzung_id = $1",
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
    ) -> anyhow::Result<Top> {
        let weight = sqlx::query!(
            "SELECT COUNT(*) FROM tops WHERE sitzung_id = $1 and top_type = $2",
            sitzung_id,
            top_type
        )
        .fetch_one(&mut **self)
        .await?
        .count;

        Ok(sqlx::query_as!(
            Top,
            "INSERT INTO tops (name, sitzung_id, weight, top_type, inhalt)
                VALUES ($1, $2, $3, $4 ,$5) RETURNING name, weight, top_type, inhalt, id",
            titel,
            sitzung_id,
            weight,
            top_type,
            *inhalt
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn add_antrag_to_top(&mut self, antrag_id: Uuid, top_id: Uuid) -> anyhow::Result<()> {
        sqlx::query!(
            "INSERT INTO antragstop (antrag_id, top_id) VALUES ($1, $2)",
            antrag_id,
            top_id
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn anträge_by_top(&mut self, top_id: Uuid) -> anyhow::Result<Vec<Antrag>> {
        Ok(sqlx::query_as!(Antrag,
            "SELECT anträge.id, anträge.antragstext, anträge.begründung, anträge.titel FROM anträge
                  JOIN antragstop ON anträge.id = antragstop.antrag_id WHERE antragstop.top_id = $1", top_id)
            .fetch_all(&mut **self)
            .await?)
    }

    async fn tops_by_sitzung(&mut self, sitzung_id: Uuid) -> anyhow::Result<Vec<Top>> {
        Ok(sqlx::query_as!(
            Top,
            "SELECT id, name, inhalt, weight, top_type FROM tops WHERE sitzung_id = $1 ORDER BY weight",
            sitzung_id
        )
        .fetch_all(&mut **self)
        .await?)
    }

    async fn get_next_sitzung(&mut self) -> anyhow::Result<Option<Sitzung>> {
        let now = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Berlin);

        let offset_int = now.time() - chrono::Utc::now().time();

        Ok(sqlx::query_as!(
            Sitzung,
            "SELECT * FROM sitzungen WHERE datum > $1 ORDER BY datum ASC",
            now.naive_utc() + offset_int
        )
        .fetch_optional(&mut **self)
        .await?)
    }

    async fn get_sitzung_today(&mut self) -> anyhow::Result<Option<Sitzung>> {
        let now = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Berlin);

        let offset_int = now.date_naive() - chrono::Utc::now().date_naive();

        Ok(sqlx::query_as!(
            Sitzung,
            "SELECT * FROM sitzungen WHERE datum > $1 ORDER BY datum ASC",
            now.naive_utc() + offset_int
        )
        .fetch_optional(&mut **self)
        .await?)
    }

    async fn update_sitzung(
        &mut self,
        id: Uuid,
        datum: NaiveDateTime,
        name: &str,
        location: &str,
    ) -> anyhow::Result<Sitzung> {
        Ok(sqlx::query_as!(
            Sitzung,
            "UPDATE sitzungen SET datum = $1, name = $2, location = $3 WHERE id = $4 RETURNING *",
            datum,
            name,
            location,
            id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_sitzung(&mut self, id: Uuid) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM tops WHERE sitzung_id = $1", id)
            .execute(&mut **self)
            .await?;
        sqlx::query!("DELETE FROM sitzungen WHERE id = $1", id)
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
    ) -> anyhow::Result<Top> {
        Ok(sqlx::query_as!(
            Top,
            "UPDATE tops SET name = $1, inhalt = $2, sitzung_id = $3, top_type = $4 WHERE id = $5 RETURNING name, inhalt, id, weight, top_type",
            titel,
            *inhalt,
            sitzung_id,
            top_type,
            id,
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_top(&mut self, id: Uuid) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM antragstop WHERE top_id = $1", id)
            .execute(&mut **self)
            .await?;
        sqlx::query!("DELETE FROM tops WHERE id = $1", id)
            .execute(&mut **self)
            .await?;
        Ok(())
    }

    async fn create_antrag_top_mapping(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> anyhow::Result<AntragTopMapping> {
        Ok(sqlx::query_as!(
            crate::domain::AntragTopMapping,
            "INSERT INTO antragstop (antrag_id, top_id) VALUES ($1, $2) RETURNING *",
            antrag_id,
            top_id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_antrag_top_mapping(
        &mut self,
        antrag_id: Uuid,
        top_id: Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            "DELETE FROM antragstop WHERE antrag_id = $1 AND top_id = $2",
            antrag_id,
            top_id
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn get_sitzung(&mut self, id: Uuid) -> anyhow::Result<Option<Sitzung>> {
        Ok(
            sqlx::query_as!(Sitzung, "SELECT * FROM sitzungen WHERE id = $1", id)
                .fetch_optional(&mut **self)
                .await?,
        )
    }
}

impl DoorStateRepo for DatabaseTransaction<'_> {
    async fn add_doorstate(
        &mut self,
        time: NaiveDateTime,
        is_open: bool,
    ) -> anyhow::Result<Doorstate> {
        Ok(sqlx::query_as!(
            Doorstate,
            "INSERT INTO doorstate (time, is_open) VALUES ($1, $2) RETURNING *",
            time,
            is_open
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn get_doorstate(&mut self, time: NaiveDateTime) -> anyhow::Result<Option<Doorstate>> {
        Ok(sqlx::query_as!(
            Doorstate,
            "SELECT * FROM doorstate WHERE time < $1 ORDER BY time DESC LIMIT 1",
            time
        )
        .fetch_optional(&mut **self)
        .await?)
    }

    async fn get_doorstate_history(&mut self) -> anyhow::Result<Option<Vec<Doorstate>>> {
        Ok(Some(
            sqlx::query_as!(Doorstate, "SELECT * FROM doorstate")
                .fetch_all(&mut **self)
                .await?,
        ))
    }
}

impl PersonRepo for DatabaseTransaction<'_> {
    async fn patch_person(&mut self, id: Uuid, name: &str) -> anyhow::Result<Person> {
        Ok(sqlx::query_as!(
            Person,
            "UPDATE person SET name = $1 WHERE id = $2 RETURNING *",
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
    ) -> anyhow::Result<PersonRoleMapping> {
        Ok(sqlx::query_as!(
            PersonRoleMapping,
            "INSERT INTO rollen (person_id, rolle, anfangsdatum, ablaufdatum) VALUES ($1, $2, $3, $4) RETURNING *",
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
    ) -> anyhow::Result<PersonRoleMapping> {
        Ok(sqlx::query_as!(
            PersonRoleMapping,
            "UPDATE rollen SET rolle = $1, anfangsdatum = $2, ablaufdatum = $3 WHERE person_id = $4 RETURNING *",
            rolle,
            anfangsdatum,
            ablaufdatum,
            person_id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_person_role_mapping(&mut self, person_id: Uuid) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM rollen WHERE person_id = $1", person_id,)
            .execute(&mut **self)
            .await?;
        Ok(())
    }

    async fn create_person(&mut self, name: &str) -> anyhow::Result<Person> {
        Ok(sqlx::query_as!(
            Person,
            "INSERT INTO person (name) VALUES ($1) ON CONFLICT(name) DO UPDATE SET name = $1 RETURNING *",
            name
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn get_persons(&mut self) -> anyhow::Result<Vec<Person>> {
        Ok(sqlx::query_as!(Person, "SELECT * FROM person")
            .fetch_all(&mut **self)
            .await?)
    }
    async fn get_person_by_role(
        &mut self,
        rolle: &str,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> anyhow::Result<Vec<Person>> {
        Ok(sqlx::query_as!(
            Person,
            "SELECT id,name FROM person
                JOIN public.rollen r on person.id = r.person_id
                WHERE r.rolle = $1 AND anfangsdatum <= $2 AND ablaufdatum >= $3",
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
    ) -> anyhow::Result<PersonRoleMapping> {
        Ok(sqlx::query_as!(
            PersonRoleMapping,
            "UPDATE rollen SET rolle = $1, anfangsdatum = $2, ablaufdatum = $3 WHERE person_id = $4 RETURNING *",
            rolle,
            anfangsdatum,
            ablaufdatum,
            person_id
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn delete_person(&mut self, id: Uuid) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM person WHERE id = $1", id)
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
    ) -> anyhow::Result<Abmeldung> {
        Ok(sqlx::query_as!(
            Abmeldung,
            "INSERT INTO abmeldungen (person_id, anfangsdatum, ablaufdatum) VALUES ($1, $2, $3) RETURNING *",
            person_id,
            anfangsdatum,
            ablaufdatum
        )
        .fetch_one(&mut **self)
        .await?)
    }

    async fn get_abmeldungen(&mut self) -> anyhow::Result<Vec<Abmeldung>> {
        Ok(sqlx::query_as!(Abmeldung, "SELECT * FROM abmeldungen")
            .fetch_all(&mut **self)
            .await?)
    }

    async fn update_person_abmeldung(
        &mut self,
        person_id: Uuid,
        anfangsdatum: NaiveDate,
        ablaufdatum: NaiveDate,
    ) -> anyhow::Result<Abmeldung> {
        Ok(sqlx::query_as!(
            Abmeldung,
            "UPDATE abmeldungen SET anfangsdatum = $1, ablaufdatum = $2 WHERE person_id = $3 RETURNING *",
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
    ) -> anyhow::Result<()> {
        sqlx::query!(
            "DELETE FROM abmeldungen WHERE person_id = $1 AND anfangsdatum = $2 AND ablaufdatum = $3",
            person_id,
            anfangsdatum,
            ablaufdatum
        )
        .execute(&mut **self)
        .await?;
        Ok(())
    }

    async fn get_abmeldungen_next_sitzung(&mut self) -> anyhow::Result<Vec<Abmeldung>> {
        let now = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Berlin);

        let offset_int = now.time() - chrono::Utc::now().time();
        let sitzung = sqlx::query_as!(
            Sitzung,
            "SELECT * FROM sitzungen WHERE datum > $1 ORDER BY datum ASC LIMIT 1",
            now.naive_utc() + offset_int
        )
        .fetch_one(&mut **self)
        .await?;
        Ok(sqlx::query_as!(
            Abmeldung,
            "SELECT * FROM abmeldungen WHERE anfangsdatum <= $1 AND ablaufdatum >= $2",
            sitzung.datum.date(),
            sitzung.datum.date()
        )
        .fetch_all(&mut **self)
        .await?)
    }
}
