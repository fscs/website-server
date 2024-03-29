use actix_web::{
    get, put,
    web::{self, Data},
    Responder, Scope,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::types::chrono;

use crate::{database::DatabasePool, domain::TopManagerRepo, web::RestStatus};

pub(crate) fn service(path: &'static str) -> Scope {
    web::scope(path)
        .service(put_doorstate)
        .service(get_doorstate)
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDoorStateParams {
    pub state: bool,
}

#[put("/")]
async fn put_doorstate(
    db: Data<DatabasePool>,
    params: web::Json<CreateDoorStateParams>,
) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| {
            let params = params.clone();
            async move {
                let now = Utc::now();
                let doorstate = transaction
                    .add_doorstate(now.naive_utc(), params.state)
                    .await?;
                Ok((doorstate, transaction))
            }
        })
        .await;

    RestStatus::ok_from_result(result)
}

#[get("/")]
async fn get_doorstate(db: Data<DatabasePool>) -> impl Responder {
    let result = db
        .transaction(move |mut transaction| async move {
            let now = Utc::now();
            let doorstate = transaction.get_doorstate(now.naive_utc()).await?;
            Ok((doorstate, transaction))
        })
        .await;

    RestStatus::ok_from_result(result)
}
