use std::{borrow::Cow, collections::HashMap, pin::Pin, sync::Arc};

use actix_utils::future::{ready, Ready};
use actix_web::{
    cookie::{Cookie, CookieJar, Key, SameSite},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::{ErrorInternalServerError, ErrorUnauthorized},
    get,
    middleware::ErrorHandlerResponse,
    web::{self, Data},
    FromRequest, HttpMessage, HttpRequest, HttpResponse, Responder,
};

use anyhow::anyhow;
use chrono::Utc;
use log::debug;
use oauth2::{
    basic::BasicClient, http::HeaderValue, reqwest::async_http_client, AuthUrl, AuthorizationCode,
    ClientId, ClientSecret, CsrfToken, RedirectUrl, RefreshToken, TokenResponse, TokenUrl,
};
use reqwest::header::{self, LOCATION};
use serde::Deserialize;
use std::future::Future;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub(crate) struct User {
    pub(crate) username: String,
    exp: i64,
    userinfo: HashMap<String, serde_json::Value>,
}

impl User {
    pub async fn from_token(
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
                log::error!("{:?}", e);
                actix_web::error::ErrorUnauthorized("Internal Error")
            })?
            .json::<HashMap<String, serde_json::Value>>()
            .await
            .map_err(|e| {
                log::error!("{:?}", e);
                actix_web::error::ErrorUnauthorized("Internal Error")
            })?;

        debug!("{:?}", userinfo);
        Ok(User {
            username: userinfo
                .get("preferred_username")
                .map(|a| a.to_string())
                .ok_or_else(|| {
                    debug!("Could not access preferred_username");
                    ErrorInternalServerError("Internal Error")
                })?,
            exp: Utc::now().timestamp() + 300,
            userinfo,
        })
    }

    pub fn is_rat(&self) -> bool {
        self.userinfo.get("realm_access").map_or(false, |r| {
            r.get("roles").map_or(false, |r| {
                r.as_array().map_or(false, |r| r.contains(&"Rat".into()))
            })
        })
    }
}

#[derive(serde::Deserialize)]
struct UserExp {
    exp: i64,
}

pub struct AuthMiddle;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddle
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware {
            service: Arc::from(service),
        }))
    }
}

pub struct AuthMiddleware<S> {
    service: Arc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let mut updated_cookies = false;
        Box::pin(async move {
            let mut jar = req.extract::<AuthCookieJar>().await?;

            let user_info = jar.user_info();

            if (jar.refresh_token().is_some() && user_info.is_none())
                || user_info.is_some_and(|u| u.exp - 30 < Utc::now().timestamp())
            {
                updated_cookies = refresh_authentication(&mut jar, &mut req).await.is_ok();
            }

            // authorized ? continue to the next middleware/ErrorHandlerResponse
            match service.call(req).await {
                Ok(mut res) if updated_cookies => {
                    res.headers_mut().append(
                        header::SET_COOKIE,
                        HeaderValue::from_str(&format!(
                            "refresh_token={}; SameSite=None; Path=/",
                            jar.refresh_token().unwrap()
                        ))
                        .unwrap(),
                    );
                    res.headers_mut().append(
                        header::SET_COOKIE,
                        HeaderValue::from_str(&format!(
                            "access_token={}; SameSite=None; Path=/",
                            jar.access_token().unwrap()
                        ))
                        .unwrap(),
                    );
                    res.headers_mut().append(
                        header::SET_COOKIE,
                        HeaderValue::from_str(&format!(
                            "user={}; SameSite=None; Path=/",
                            jar.jar.get("user").unwrap().value()
                        ))
                        .unwrap(),
                    );
                    Ok(res)
                }
                c => c,
            }
        })
    }
}

async fn refresh_authentication(
    jar: &mut AuthCookieJar,
    req: &mut ServiceRequest,
) -> anyhow::Result<()> {
    debug!("Refreshing user {:?}", jar.user_info());
    let refresh = jar
        .refresh_token()
        .ok_or(anyhow!("Could not access refresh token"))?;
    let oauth_client = req.app_data::<Data<OauthClient>>().unwrap();

    let token = oauth_client
        .client
        .exchange_refresh_token(&RefreshToken::new(refresh.to_owned()))
        .request_async(async_http_client)
        .await?;

    jar.set_refresh_token(token.refresh_token().map_or("/", |a| a.secret()));
    jar.set_access_token(token.access_token().secret());

    let user = User::from_token(token.access_token().secret(), oauth_client)
        .await
        .map_err(|e| anyhow!("{:?}", e))?;
    jar.set_user_info(&user);

    let cookie_header = req
        .headers_mut()
        .get_mut(header::COOKIE)
        .ok_or(anyhow!("Could not read Cookies"))?;

    let cookie_header_str = cookie_header.to_str()?;

    let cookie_header = HeaderValue::from_str(
        &(cookie_header_str
            .split(';')
            .filter_map(|c| match c.split_once('=') {
                Some((c, _)) if c.contains("refresh_token") => None,
                Some((c, _)) if c.contains("access_token") => None,
                Some((c, _)) if c.contains("user") => None,
                _ => Some(c.to_owned()),
            })
            .fold(
                format!(
                    "refresh_token={}; access_token={}; user={}",
                    jar.refresh_token()
                        .ok_or(anyhow!("Could not access refresh_token"))?,
                    jar.access_token()
                        .ok_or(anyhow!("Could not access access_token"))?,
                    jar.jar
                        .get("user")
                        .ok_or(anyhow!("Could not access user"))?
                        .value()
                ),
                |a, b| a + ";" + &b,
            )
            + ";"),
    )?;

    req.headers_mut().insert(header::COOKIE, cookie_header);
    req.extensions_mut().clear();

    Ok(())
}

pub(crate) struct OauthClient {
    client: BasicClient,
    reqwest_client: reqwest::Client,
    user_info: String,
    singning_key: Key,
}

struct AuthCookieJar {
    jar: CookieJar,
    key: Key,
}

impl AuthCookieJar {
    fn access_token(&self) -> Option<&str> {
        self.jar.get("access_token").map(|c| c.value())
    }

    fn set_access_token(&mut self, value: &str) {
        let mut cookie = Cookie::new("access_token", value.to_string());
        cookie.set_path("/");
        cookie.set_same_site(SameSite::None);
        self.jar.add(cookie)
    }

    fn refresh_token(&self) -> Option<&str> {
        self.jar.get("refresh_token").map(|c| c.value())
    }

    fn set_refresh_token(&mut self, value: &str) {
        let mut cookie = Cookie::new("refresh_token", value.to_string());
        cookie.set_path("/");
        cookie.set_same_site(SameSite::None);
        self.jar.add(cookie);
    }

    fn user_info(&self) -> Option<User> {
        self.jar
            .signed(&self.key)
            .get("user")
            .and_then(|c| serde_json::from_str::<User>(c.value()).ok())
            .filter(|u| u.exp > Utc::now().timestamp())
    }

    fn set_user_info(&mut self, user: &User) {
        let mut cookie = Cookie::new("user", serde_json::to_string(&user).unwrap());
        cookie.set_path("/");
        cookie.set_same_site(SameSite::None);
        self.jar.signed_mut(&self.key).add(cookie);
    }

    fn delta(&self) -> impl Iterator<Item = &Cookie<'static>> {
        self.jar.delta()
    }
}

impl FromRequest for AuthCookieJar {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let key = &req.app_data::<Data<OauthClient>>().unwrap().singning_key;
            let mut jar = CookieJar::new();

            let Ok(cookies) = req.cookies() else {
                return Err(actix_web::error::ErrorBadRequest("Can not Access Cookies"));
            };

            for c in cookies.iter() {
                jar.add_original(c.clone())
            }

            Ok(AuthCookieJar {
                jar,
                key: key.clone(),
            })
        })
    }
}

impl FromRequest for User {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let jar = AuthCookieJar::extract(&req).await?;

            if let Some(user) = jar.user_info() {
                Ok(user)
            } else if let Some(access_token) = jar.access_token() {
                let oauth_client = req.app_data::<Data<OauthClient>>().ok_or(
                    actix_web::error::ErrorInternalServerError(
                        "Broken config please Contact an Admin",
                    ),
                )?;
                User::from_token(access_token, oauth_client)
                    .await
                    .map_err(|e| {
                        debug!("{:?}", e);
                        ErrorUnauthorized("Invalid access_token")
                    })
            } else {
                debug!("Could not access user info");
                Err(actix_web::error::ErrorUnauthorized(
                    "Could not access user info",
                ))
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
    let singning_key = std::env::var("SIGNING_KEY").expect("No SIGNING_KEY set");

    OauthClient {
        client: BasicClient::new(ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://login.inphima.de/auth/realms/FSCS-Intern/protocol/openid-connect/auth".to_string()).unwrap(),
            Some(TokenUrl::new("https://login.inphima.de/auth/realms/FSCS-Intern/protocol/openid-connect/token".to_string()).unwrap())),
            reqwest_client: reqwest::Client::new(),
            user_info: "https://login.inphima.de/auth/realms/FSCS-Intern/protocol/openid-connect/userinfo".to_string(),
            singning_key: Key::from(singning_key.as_bytes())
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
    //rmove cookies and redirect to /
    let mut cookie_at = Cookie::build("access_token", "").path("/").finish();
    let mut cookie_rt = Cookie::build("refresh_token", "").path("/").finish();
    let mut cookie_u = Cookie::build("user", "").path("/").finish();
    cookie_at.make_removal();
    cookie_rt.make_removal();
    cookie_u.make_removal();

    HttpResponse::Found()
        .cookie(cookie_u)
        .cookie(cookie_at)
        .cookie(cookie_rt)
        .append_header((actix_web::http::header::LOCATION, "/"))
        .finish()
}

#[get("/callback/")]
async fn callback(
    oauth_client: web::Data<OauthClient>,
    query: web::Query<AuthRequest>,
    mut auth_jar: AuthCookieJar,
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
        return HttpResponse::InternalServerError().body("Could not get token from Provider");
    };

    let access_token = token.access_token().secret();
    let refresh_token = match token.refresh_token() {
        Some(refresh_token) => refresh_token.secret(),
        None => "",
    };

    auth_jar.set_access_token(access_token);
    auth_jar.set_refresh_token(refresh_token);

    let Ok(user) = User::from_token(access_token, &oauth_client).await else {
        return HttpResponse::InternalServerError().body("Could not access user info");
    };
    auth_jar.set_user_info(&user);

    let mut ressponse_builder = HttpResponse::Found();
    ressponse_builder.append_header((actix_web::http::header::LOCATION, path));

    //info!("{:?}", auth_jar.delta());
    for cookie in auth_jar.delta() {
        ressponse_builder.cookie(cookie.clone());
    }

    ressponse_builder.finish()
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
