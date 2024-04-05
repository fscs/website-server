use std::pin::Pin;

use actix_web::{get, web, FromRequest, HttpResponse, Responder};
use serde::Deserialize;
use std::future::Future;
use oauth2::{basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenResponse, TokenUrl};

struct User {
    username: String
}

struct OauthClient {
    client: BasicClient
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    // The TIMESTAMP at which the token expires
    exp: u64,
    // The TIMESTAMP at which the token was issued
    iat: u64,
    
}

impl FromRequest for User {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self, Self::Error>>>>;

    fn from_request(req: &actix_web::HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        req.cookie("access_token")
            .map(|cookie| cookie.value().to_string())
            .map(|access_token| async move {
                let token = TokenResponse::new(access_token);
                let user = User::from_token(token).await?;
                Ok(user)
            })
            .unwrap_or_else(|| {
                Box::pin(async {
                    Err(actix_web::error::ErrorUnauthorized("No access token found"))
                })
            })
        Box::pin(async {
            Ok(User {
                username: "test".to_string()
            }
        )})
        
    }
}

#[derive(Deserialize)]
struct AuthRequest {
    code: String,
    state: String,
}

pub(crate) fn service(path: &'static str) -> actix_web::Scope {
    let client_id = std::env::var("CLIENT_ID").expect("No CLIENT ID set");
    let client_secret = std::env::var("CLIENT_SECRET").unwrap();

    web::scope(path)
        .app_data(web::Data::new(OauthClient {
            client: BasicClient::new(ClientId::new(client_id),
                Some(ClientSecret::new(client_secret)),
                AuthUrl::new("https://login.inphima.de/auth/realms/FSCS-Intern/protocol/openid-connect/auth".to_string()).unwrap(),
                Some(TokenUrl::new("https://login.inphima.de/auth/realms/FSCS-Intern/protocol/openid-connect/token".to_string()).unwrap()),
                )
            .set_redirect_uri(RedirectUrl::new("http://localhost:8080/auth/callback".to_string()).unwrap())
        }))
        .service(login)
        .service(callback)
    
}

#[get("/login")]
async fn login(oauth_client: web::Data<OauthClient>) -> impl Responder {

    let (auth_url, _csrf_token) = &oauth_client.client
        .authorize_url(CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("openid".to_string()))
        .url();

    HttpResponse::Found()
        .append_header((actix_web::http::header::LOCATION, auth_url.to_string()))
        .finish()
}

#[get("/logout")]
async fn logout() -> impl Responder {
    HttpResponse::Ok()
        .cookie(actix_web::cookie::Cookie::build("access_token", "").finish())
        .finish()
}

#[get("/callback")]
async fn callback(
    oauth_client: web::Data<OauthClient>,
    query: web::Query<AuthRequest>,
) -> impl Responder {
    let code = AuthorizationCode::new(query.code.clone());
    let _state = query.state.clone();

    let Ok(token) = oauth_client.client
        .exchange_code(code)
        // Set the PKCE code verifier.
        .request_async(async_http_client)
        .await else {
            return HttpResponse::Unauthorized().finish();
    };

    let access_token = token.access_token().secret();

    HttpResponse::Ok()
        .cookie(actix_web::cookie::Cookie::build("access_token", access_token).finish())
        .finish()
}