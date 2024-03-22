use std::pin::Pin;

use actix_web::FromRequest;
use std::future::Future;


struct User {
    username: String
}

impl FromRequest for User {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self, Self::Error>>>>;

    fn from_request(req: &actix_web::HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        

        Box::pin(async {
            Ok(User {
                username: "test".to_string()
            }
        )})
        
    }
}