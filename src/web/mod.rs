use crate::database::{DatabasePool, DatabaseTransaction};
use actix_web::body::BoxBody;
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest, HttpResponse, Responder};
use lazy_static::lazy_static;
use std::future::Future;
use std::pin::Pin;
use tera::Context;

pub(crate) mod calendar;
pub(crate) mod topmanager;

#[derive(Clone)]
struct TerraResponse {
    template: &'static str,
    context: Context,
}

impl FromRequest for DatabaseTransaction<'static> {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            if let Some(pool) = req.app_data::<DatabasePool>() {
                match pool.start_transaction().await {
                    Ok(transaction) => Ok(transaction),
                    Err(err) => {
                        log::debug!("{:?}", err);
                        Err(actix_web::error::ErrorInternalServerError(
                            "Could not access Database",
                        ))
                    }
                }
            } else {
                log::debug!("Failed to extract the DatabasePool");
                Err(actix_web::error::ErrorInternalServerError(
                    "Requested application data is not configured correctly",
                ))
            }
        })
    }
}

impl Responder for TerraResponse {
    type Body = BoxBody;
    fn respond_to(self, _: &HttpRequest) -> HttpResponse<Self::Body> {
        lazy_static! {
            static ref TERA: tera::Tera = {
                let current_dir = crate::get_base_dir().unwrap();
                tera::Tera::new(&(current_dir + "/templates/**/*")).unwrap()
            };
        }

        let body = TERA.render(self.template, &self.context).unwrap();
        HttpResponse::Ok().body(body)
    }
}
