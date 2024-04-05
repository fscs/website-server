use std::{borrow::Cow, collections::HashMap, pin::Pin};

use actix_web::{
    cookie::{Cookie, SameSite},
    dev::ServiceResponse,
    error::ErrorInternalServerError,
    get,
    middleware::ErrorHandlerResponse,
    web::{self, Data},
    FromRequest, HttpRequest, HttpResponse, Responder,
};
use log::error;
use oauth2::{
    basic::BasicClient, http::HeaderValue, reqwest::async_http_client, AuthUrl, AuthorizationCode,
    ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenResponse, TokenUrl,
};
use reqwest::header::LOCATION;
use serde::Deserialize;
use std::future::Future;

#[derive(serde::Deserialize)]
pub(crate) struct User {
    pub(crate) username: String,
    userinfo: HashMap<String, serde_json::Value>,
}

impl User {
    async fn from_token(
        access_token: &str,
        oauth_client: &OauthClient,
    ) -> Result<Self, actix_web::Error> {
        let userinfo = oauth_client
            .reqwest_client
            .get(oauth_client.user_info.to_owned())
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| {
                error!("{:?}", e);
                actix_web::error::ErrorUnauthorized("Internal Error")
            })?
            .json::<HashMap<String, serde_json::Value>>()
            .await
            .map_err(|e| {
                error!("{:?}", e);
                actix_web::error::ErrorUnauthorized("Internal Error")
            })?;

        Ok(User {
            username: userinfo
                .get("preferred_username")
                .map(|a| a.to_string())
                .ok_or(ErrorInternalServerError("Internal Error"))?,
            userinfo,
        })
    }
}

pub(crate) struct OauthClient {
    client: BasicClient,
    reqwest_client: reqwest::Client,
    user_info: String,
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
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let oauth_client = req.app_data::<Data<OauthClient>>().unwrap();

            match req.cookie("access_token") {
                Some(access_token) => User::from_token(access_token.value(), &*oauth_client).await,
                None => Err(actix_web::error::ErrorUnauthorized("missing access_token")),
            }
        })
    }
}

#[derive(Deserialize, Debug)]
struct AuthRequest {
    code: String,
    state: String,
    path: Option<String>,
}

pub(crate) fn oauth_client() -> OauthClient {
    let client_id = std::env::var("CLIENT_ID").expect("No CLIENT ID set");
    let client_secret = std::env::var("CLIENT_SECRET").unwrap();

    OauthClient {
        client: BasicClient::new(ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://login.inphima.de/auth/realms/FSCS-Intern/protocol/openid-connect/auth".to_string()).unwrap(),
            Some(TokenUrl::new("https://login.inphima.de/auth/realms/FSCS-Intern/protocol/openid-connect/token".to_string()).unwrap())),
            reqwest_client: reqwest::Client::new(),
            user_info: "https://login.inphima.de/auth/realms/FSCS-Intern/protocol/openid-connect/userinfo".to_string()
        }
}

pub(crate) fn service(path: &'static str) -> actix_web::Scope {
    web::scope(path)
        .service(login)
        .service(callback)
        .service(logout)
}

#[derive(serde::Deserialize, Debug)]
struct PathParam {
    path: Option<String>,
}

fn redirect_url<'a>(path: &str, request: HttpRequest) -> Cow<'a, RedirectUrl> {
    let host = request.connection_info().host().to_string();
    let scheme = request.connection_info().scheme().to_string();

    std::borrow::Cow::Owned(
        RedirectUrl::new(format!(
            "{}://{}/auth/callback/?path={}",
            scheme, host, path
        ))
        .unwrap(),
    )
}

#[get("/login/")]
async fn login(
    oauth_client: web::Data<OauthClient>,
    path: web::Query<PathParam>,
    request: HttpRequest,
) -> impl Responder {
    let path = path.into_inner().path.unwrap_or("/".to_string());

    let (ref mut auth_url, csrf_token) = &mut oauth_client
        .client
        .authorize_url(CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("openid".to_string()))
        .set_redirect_uri(redirect_url(&path, request))
        .url();

    HttpResponse::Found()
        .append_header((actix_web::http::header::LOCATION, auth_url.to_string()))
        .cookie(
            Cookie::build("csrf", csrf_token.secret())
                .same_site(SameSite::None)
                .path("/auth/")
                .finish(),
        )
        .finish()
}

#[get("/logout/")]
async fn logout() -> impl Responder {
    let mut cookie = Cookie::build("access_token", "").path("/").finish();
    cookie.make_removal();
    HttpResponse::Ok().cookie(cookie).finish()
}

#[get("/callback/")]
async fn callback(
    oauth_client: web::Data<OauthClient>,
    query: web::Query<AuthRequest>,
    request: HttpRequest,
) -> impl Responder {
    let code = AuthorizationCode::new(query.code.clone());
    let state = query.state.clone();
    let Some(csrf_token) = request.cookie("csrf") else {
        return HttpResponse::Unauthorized().body("missing csrf token");
    };

    if state != csrf_token.value() {
        return HttpResponse::Unauthorized().body("wrong csrf state");
    }

    let path = query.path.clone().unwrap_or("/".to_string());

    let Ok(token) = oauth_client
        .client
        .exchange_code(code)
        .set_redirect_uri(redirect_url(&path, request))
        // Set the PKCE code verifier.
        .request_async(async_http_client)
        .await
    else {
        return HttpResponse::Unauthorized().body("Could not get token from Provider");
    };

    let access_token = token.access_token().secret();

    HttpResponse::Found()
        .append_header((actix_web::http::header::LOCATION, path))
        .cookie(
            actix_web::cookie::Cookie::build("access_token", access_token)
                .expires(None)
                .same_site(SameSite::Lax)
                .path("/")
                .finish(),
        )
        .finish()
}

pub fn not_authorized<B>(
    res: actix_web::dev::ServiceResponse<B>,
) -> actix_web::Result<actix_web::middleware::ErrorHandlerResponse<B>> {
    let (req, mut res) = res.into_parts();
    let path = req.path().to_string();
    *res.status_mut() = actix_web::http::StatusCode::FOUND;

    let mut res = ServiceResponse::new(req, res).map_into_left_body();

    res.headers_mut().append(
        LOCATION,
        HeaderValue::from_str(&format!("/auth/login/?path={path}"))
            .map_err(|_| ErrorInternalServerError("Invalid Path"))?,
    );

    Ok(ErrorHandlerResponse::Response(res))
}
