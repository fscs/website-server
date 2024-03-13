use crate::database::DatabasePool;
use actix_web::{
    delete, get, patch, post, put,
    web::{self, Data},
    HttpResponse, Responder, Scope,
};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Row};
use uuid::Uuid;

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(create_antrag)
        .service(update_antrag)
        .service(get_anträge)
        .service(delete_antrag)
        .service(get_sitzungen)
        .service(tops_by_sitzung)
        .service(anträge_by_top)
        .service(anträge_by_sitzung)
        .service(create_sitzung)
}

#[derive(Debug, Deserialize)]
pub struct CreateAntragParams {
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
    pub antragssteller: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAntragParams {
    pub uuid: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteAntragParams {
    pub id: Uuid,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Antrag {
    pub id: Uuid,
    pub titel: String,
    pub antragstext: String,
    pub begründung: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Top {
    pub id: Uuid,
    pub position: i32,
    pub sitzung_id: Uuid,
    pub name: String,
    pub inhalt: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTopParams {
    pub titel: String,
    pub sitzung_id: Uuid,
    pub inhalt: Option<serde_json::Value>,
    pub position: i32,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Sitzung {
    pub id: Uuid,
    pub datum: chrono::NaiveDateTime,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct CreateSitzungParams {
    pub datum: chrono::NaiveDateTime,
    pub name: String,
}

#[derive(Debug, Serialize, FromRow)]
struct Person {
    pub id: Uuid,
    pub name: String,
}
#[put("/antrag")]
async fn create_antrag(
    db: Data<DatabasePool>,
    params: web::Json<CreateAntragParams>,
) -> impl Responder {
    let result = sqlx::query_as::<_, Antrag>(
        "INSERT INTO anträge (titel, antragstext, begründung) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(&params.titel)
    .bind(&params.antragstext)
    .bind(&params.begründung)
    .fetch_one(db.pool())
    .await;

    let antrag = match result {
        Ok(antrag) => antrag,
        Err(e) => {
            log::error!("Failed to create Antrag: {:?}", e);
            return "Failed to create Antrag";
        }
    };

    let result = sqlx::query_as::<_, Person>(
        "Insert into person (name) VALUES ($1) ON CONFLICT (name) Do NOTHING RETURNING *",
    )
    .bind(&params.antragssteller)
    .fetch_one(db.pool())
    .await;

    let person = match result {
        Ok(person) => person,
        Err(e) => {
            log::error!("Failed to create Person: {:?}", e);
            return "Failed to create Person";
        }
    };

    let result =
        sqlx::query("INSERT INTO antragsstellende (antrags_id, person_id) VALUES ($1, $2)")
            .bind(antrag.id)
            .bind(person.id)
            .execute(db.pool())
            .await;

    match result {
        Ok(_) => "Antrag erstellt",
        Err(e) => {
            log::error!("Failed to create Antragsstellende: {:?}", e);
            return "Failed to create Antragsstellende";
        }
    };

    let now = chrono::Utc::now();

    //check if there is a sitzung in the future
    let sitzungen = sqlx::query_as::<_, Sitzung>("SELECT * FROM sitzungen WHERE datum > $1")
        .bind(now)
        .fetch_all(db.pool())
        .await;

    match &sitzungen {
        Ok(sitzungen) => {
            if sitzungen.is_empty() {
                return "Noch keine Sitzung geplant";
            }
        }
        Err(e) => {
            log::error!("Failed to get Sitzungen: {:?}", e);
            return "Failed to get Sitzungen";
        }
    }

    let sitzung = sitzungen.as_ref().unwrap().first().unwrap();

    //get the last created top
    let top = sqlx::query_as::<_, Top>("SELECT * FROM tops ORDER BY id DESC LIMIT 1")
        .fetch_optional(db.pool())
        .await;

    let top = match top {
        Ok(top) => top,
        Err(e) => {
            log::error!("Failed to get Top: {:?}", e);
            return "Failed to get Top";
        }
    };

    //create new top
    let result = sqlx::query_as::<_, Top>(
        "INSERT INTO tops (name, sitzung_id, position) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(&params.titel)
    .bind(sitzung.id)
    .bind(top.map_or(0, |x| x.position + 1))
    .fetch_one(db.pool())
    .await;

    let top = match result {
        Ok(top) => top,
        Err(e) => {
            log::error!("Failed to create Top: {:?}", e);
            return "Failed to create Top";
        }
    };

    //create mapping between top and antrag
    let result = sqlx::query("INSERT INTO antragstop (antrag_id, top_id) VALUES ($1, $2)")
        .bind(antrag.id)
        .bind(top.id)
        .execute(db.pool())
        .await;

    match result {
        Ok(_) => "Antrag erstellt",
        Err(e) => {
            log::error!("Failed to create Antragmapping: {:?}", e);
            "Failed to create Antragmapping"
        }
    }
}

#[patch("/antrag")]
async fn update_antrag(
    db: Data<DatabasePool>,
    params: web::Json<UpdateAntragParams>,
) -> impl Responder {
    let result = sqlx::query(
        "UPDATE anträge SET titel = $1, antragstext = $2, begründung = $3 WHERE id = $4",
    )
    .bind(&params.titel)
    .bind(&params.antragstext)
    .bind(&params.begründung)
    .bind(params.uuid)
    .execute(db.pool())
    .await;
    match result {
        Ok(_) => "Antrag geändert",
        Err(e) => {
            log::error!("Failed to update Antrag: {:?}", e);
            "Failed to update Antrag"
        }
    }
}

#[get("/get_anträge")]
async fn get_anträge(db: Data<DatabasePool>) -> impl Responder {
    match sqlx::query_as::<_, Antrag>("SELECT * FROM anträge")
        .fetch_all(db.pool())
        .await
    {
        Ok(anträge) => HttpResponse::Ok().json(anträge),
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get Anträge: {:?}", e)),
    }
}

#[delete("/antrag")]
async fn delete_antrag(
    db: Data<DatabasePool>,
    params: web::Query<DeleteAntragParams>,
) -> impl Responder {
    let result = sqlx::query("DELETE FROM anträge WHERE id = $1")
        .bind(params.id)
        .execute(db.pool());
    match result.await {
        Ok(_) => "Antrag gelöscht",
        Err(e) => {
            log::error!("Failed to delete Antrag: {:?}", e);
            "Failed to delete Antrag"
        }
    }
}

#[get("/sitzungen")]
async fn get_sitzungen(db: Data<DatabasePool>) -> impl Responder {
    match sqlx::query_as::<_, Sitzung>("SELECT * FROM sitzungen")
        .fetch_all(db.pool())
        .await
    {
        Ok(sitzungen) => HttpResponse::Ok().json(sitzungen),
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get Sitzungen: {:?}", e)),
    }
}

#[put("/sitzung")]
async fn create_sitzung(
    db: Data<DatabasePool>,
    params: web::Json<CreateSitzungParams>,
) -> impl Responder {
    let result = sqlx::query("INSERT INTO sitzungen (datum, name) VALUES ($1, $2)")
        .bind(params.datum)
        .bind(&params.name)
        .execute(db.pool());
    match result.await {
        Ok(_) => "Sitzung erstellt",
        Err(e) => {
            log::error!("Failed to create Sitzung: {:?}", e);
            "Failed to create Sitzung"
        }
    }
}

#[put("/top")]
async fn create_top(db: Data<DatabasePool>, params: web::Json<CreateTopParams>) -> impl Responder {
    let result = sqlx::query("INSERT INTO tops (name, sitzung_id, inhalt) VALUES ($1, $2, $3")
        .bind(&params.titel)
        .bind(params.sitzung_id)
        .bind(&params.inhalt)
        .execute(db.pool());
    match result.await {
        Ok(_) => "Top erstellt",
        Err(e) => {
            log::error!("Failed to create Top: {:?}", e);
            "Failed to create Top"
        }
    }
}

#[get("/sitzungen/{id}/tops")]
async fn tops_by_sitzung(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let topids = sqlx::query("SELECT top_id FROM sitzungstop WHERE sitzung_id = $1")
        .bind(*id)
        .fetch_all(db.pool())
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| row.get::<Uuid, _>("top_id"))
                .collect::<Vec<_>>()
        });

    match topids {
        Ok(topids) => {
            let tops = sqlx::query_as::<_, Top>("SELECT * FROM tops WHERE id = ANY($1)")
                .bind(&topids)
                .fetch_all(db.pool())
                .await;
            match tops {
                Ok(tops) => HttpResponse::Ok().json(tops),
                Err(e) => HttpResponse::NotFound().json(format!("Failed to get Tops: {:?}", e)),
            }
        }
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get TopIds: {:?}", e)),
    }
}

#[get("/tops/{topid}/anträge")]
async fn anträge_by_top(db: Data<DatabasePool>, topid: web::Path<Uuid>) -> impl Responder {
    let anträge = sqlx::query_as::<_, Antrag>(
        "SELECT * From anträge Join antragstop ON anträge.id = antragstop.antrag_id WHERE top_id = $1",
    )
    .bind(*topid)
    .fetch_all(db.pool())
    .await;
    match anträge {
        Ok(anträge) => HttpResponse::Ok().json(anträge),
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get Anträge: {:?}", e)),
    }
}

#[get("/sitzungen/{id}/anträge")]
async fn anträge_by_sitzung(db: Data<DatabasePool>, id: web::Path<Uuid>) -> impl Responder {
    let anträge = sqlx::query_as::<_, Antrag>(
        "SELECT * FROM anträge
        JOIN antragstop ON anträge.id = antragstop.antrag_id
        JOIN public.sitzungen s2 on tops.sitzung_id = s2.id
        WHERE s2.id = $1",
    )
    .bind(*id)
    .fetch_all(db.pool())
    .await;

    match anträge {
        Ok(anträge) => HttpResponse::Ok().json(anträge),
        Err(e) => HttpResponse::NotFound().json(format!("Failed to get Anträge: {:?}", e)),
    }
}
