use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    str::FromStr,
    sync::{Arc, LazyLock},
};

use actix_utils::future::{ready, Ready};
use actix_web::{
    cookie::{time::Duration, Cookie, CookieJar, Key, SameSite},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::{self, ErrorUnauthorized},
    get,
    http::header,
    web::{self, Data},
    FromRequest, HttpMessage, HttpRequest, HttpResponse, Responder,
};
use chrono::Utc;
use log::{debug, info};
use oauth2::{
    basic::BasicClient, http::HeaderValue, reqwest::async_http_client, AuthUrl, AuthorizationCode,
    ClientId, ClientSecret, CsrfToken, RedirectUrl, RefreshToken, TokenResponse, TokenUrl,
};
use regex::Regex;
use serde::Deserialize;

use crate::{
    database::DatabaseTransaction,
    domain::{
        self,
        persons::{Person, PersonRepo},
        Capability,
    },
    ARGS,
};

pub mod capability; 

const COOKIE_MAX_AGE: Duration = Duration::days(30);

static AUTH_COOKIE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::from_str("(refresh_token=[.--;]*;)|(access_token=[.--;]*;)|(user=[.--;]*;)").unwrap()
});

/// Maps Capabilites to Roles that have them
static CAPABILITY_SET: LazyLock<HashMap<Capability, HashSet<String>>> = LazyLock::new(|| {
    let mut set: HashMap<Capability, HashSet<String>> = HashMap::new();

    for (role, capabilities_str) in &ARGS.groups {
        let capabilities = capabilities_str
            .split(',')
            .map(|s| Capability::from_str(s).unwrap());

        for cap in capabilities {
            set.entry(cap)
                .or_insert_with_key(|_| HashSet::new())
                .insert(role.clone());
        }
    }

    set
});

fn exp_default() -> i64 {
    Utc::now().timestamp() + 300
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct User {
    #[serde(skip, default = "exp_default")]
    pub exp: i64,

    pub name: String,
    pub preferred_username: String,
    pub sub: String,
    #[serde(default)]
    pub groups: Vec<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl User {
    pub async fn from_token(
        access_token: &str,
        oauth_client: &OauthClient,
    ) -> Result<Self, actix_web::Error> {
        oauth_client
            .reqwest_client
            .get(oauth_client.user_info.clone())
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| {
                log::error!("{:?}", e);
                actix_web::error::ErrorUnauthorized("Internal Error")
            })?
            .json()
            .await
            .map_err(|e| {
                log::error!("{:?}", e);
                actix_web::error::ErrorUnauthorized("Internal Error")
            })
    }

    pub async fn query_person(&self, repo: &mut impl PersonRepo) -> domain::Result<Person> {
        let user_name = format!("{}-{}", ARGS.oauth_source_name.as_str(), self.sub.as_str());
        repo.person_by_user_name(user_name.as_str())
            .await?
            .ok_or_else(|| {
                domain::Error::Message(
                    "no corresponding person found in database, try logging out and back in again"
                        .to_string(),
                )
            })
    }

    pub fn has_capability(&self, cap: Capability) -> bool {
        let fun = |allowed_groups: &HashSet<String>| {
            self.groups
                .iter()
                .any(|group| allowed_groups.contains(group))
        };

        CAPABILITY_SET.get(&cap).map(fun).unwrap_or_else(|| {
            CAPABILITY_SET
                .get(&Capability::Admin)
                .map(fun)
                .unwrap_or(false)
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
            if let Some(user) = req.extensions().get::<User>() {
                Ok(user.to_owned())
            } else {
                Err(ErrorUnauthorized("Could not obtain user info"))
            }
        })
    }
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
            let oauth_client =
                req.app_data::<Data<OauthClient>>()
                    .ok_or(domain::Error::Message(
                        "oauth client is not configured".to_string(),
                    ))?;

            // try obtaining a user
            //
            // - if there is a user, but it is expired, get a new one
            // - if there is a user, just use that one
            // - if there is none, and the Authorization header is set, try get a user using that
            // - if there is none, but we have a refresh_token, use that to get a new one
            // - if there is none, but we have an access_token, use that to get a new one
            // - otherwise, just give up
            //
            let maybe_user = match jar.user_info() {
                Some(user) if user.exp - 30 < Utc::now().timestamp() => {
                    let maybe_new_user = refresh_authentication(&mut jar, &mut req).await;
                    updated_cookies = true;
                    maybe_new_user.ok()
                }
                Some(user) => Some(user),
                None if req.headers().contains_key("Authorization") => {
                    if let Some(token) =
                        token_from_auth_header(req.headers().get("Authorization").unwrap())
                    {
                        User::from_token(token, oauth_client).await.ok()
                    } else {
                        None
                    }
                }
                None if jar.refresh_token().is_some() => {
                    let maybe_new_user = refresh_authentication(&mut jar, &mut req).await;
                    updated_cookies = true;
                    maybe_new_user.ok()
                }
                None if jar.access_token().is_some() => {
                    User::from_token(jar.access_token().unwrap(), oauth_client)
                        .await
                        .ok()
                }
                None => None,
            };

            if let Some(user) = maybe_user {
                debug!("user {} aquired", user.preferred_username);
                req.extensions_mut().insert(user);
            }

            // authorized ? continue to the next middleware/ErrorHandlerResponse
            match service.call(req).await {
                Ok(mut res) if updated_cookies => {
                    for cookie in [
                        jar.refresh_token().map(|r| format!("refresh_token={r}")),
                        jar.access_token().map(|a| format!("access_token={a}")),
                        jar.inner
                            .get("user")
                            .map(Cookie::value)
                            .map(|u| format!("user={u}")),
                    ]
                    .into_iter()
                    .flatten()
                    .filter_map(|cookie| {
                        HeaderValue::from_str(&format!(
                            "{}; SameSite=None; Path=/; Secure; Max-Age={};",
                            cookie, COOKIE_MAX_AGE
                        ))
                        .ok()
                    }) {
                        res.headers_mut().append(header::SET_COOKIE, cookie);
                    }
                    Ok(res)
                }
                c => c,
            }
        })
    }
}

fn token_from_auth_header(header: &HeaderValue) -> Option<&str> {
    let header_str = header.to_str().ok()?;

    let parts: Vec<&str> = header_str.split_whitespace().collect();
    if parts.len() != 2 || parts[0] != "Bearer" {
        return None;
    }

    Some(parts[1])
}

async fn refresh_authentication(
    jar: &mut AuthCookieJar,
    req: &mut ServiceRequest,
) -> domain::Result<User> {
    info!("Refreshing user {:?}", jar.user_info());
    let refresh = jar.refresh_token().ok_or(domain::Error::Message(
        "Could not access refresh token".to_string(),
    ))?;

    let oauth_client = req
        .app_data::<Data<OauthClient>>()
        .ok_or(domain::Error::Message(
            "oauth client is not configured".to_string(),
        ))?;

    let token = oauth_client
        .client
        .exchange_refresh_token(&RefreshToken::new(refresh.to_owned()))
        .request_async(async_http_client)
        .await
        .map_err(|e| domain::Error::Message(format!("{:?}", e)))?;

    let refresh_token = token.refresh_token().map_or("/", |a| a.secret());
    let access_token = token.access_token().secret();
    jar.set_refresh_token(refresh_token);
    jar.set_access_token(access_token);

    let user = User::from_token(token.access_token().secret(), oauth_client)
        .await
        .map_err(|e| domain::Error::Message(format!("{:?}", e)))?;

    let userinfo = jar.set_user_info(&user);

    let cookie_header = req
        .headers_mut()
        .get_mut(header::COOKIE)
        .ok_or(domain::Error::Message("Could not read Cookies".to_string()))?
        .to_str()
        .map_err(|e| domain::Error::Message(format!("{:?}", e)))?;

    let cookie_header = AUTH_COOKIE_REGEX.replace_all(cookie_header, "");

    let cookie_header = format!(
        "{}refresh_token={};access_token={};user={}",
        cookie_header, refresh_token, access_token, userinfo
    );

    req.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&cookie_header)
            .map_err(|e| domain::Error::Message(format!("{:?}", e)))?,
    );
    req.extensions_mut().clear();

    Ok(user)
}

pub(crate) struct OauthClient {
    client: BasicClient,
    reqwest_client: reqwest::Client,
    user_info: String,
    singning_key: Key,
}

struct AuthCookieJar {
    inner: CookieJar,
    key: Key,
}

impl AuthCookieJar {
    fn access_token(&self) -> Option<&str> {
        self.inner
            .get("access_token")
            .map(actix_web::cookie::Cookie::value)
    }

    fn set_access_token(&mut self, value: &str) {
        let mut cookie = Cookie::new("access_token", value.to_string());
        cookie.set_path("/");
        cookie.set_same_site(SameSite::None);
        cookie.set_max_age(COOKIE_MAX_AGE);
        self.inner.add(cookie);
    }

    fn refresh_token(&self) -> Option<&str> {
        self.inner
            .get("refresh_token")
            .map(actix_web::cookie::Cookie::value)
    }

    fn set_refresh_token(&mut self, value: &str) {
        let mut cookie = Cookie::new("refresh_token", value.to_string());
        cookie.set_path("/");
        cookie.set_same_site(SameSite::None);
        cookie.set_max_age(COOKIE_MAX_AGE);
        self.inner.add(cookie);
    }

    fn user_info(&self) -> Option<User> {
        self.inner
            .signed(&self.key)
            .get("user")
            .and_then(|c| serde_json::from_str::<User>(c.value()).ok())
            .filter(|u| u.exp > Utc::now().timestamp())
    }

    fn set_user_info(&mut self, user: &User) -> &str {
        let user_str = serde_json::to_string(&user).unwrap();

        let mut cookie = Cookie::new("user", user_str.clone());

        cookie.set_path("/");
        cookie.set_max_age(COOKIE_MAX_AGE);
        cookie.set_same_site(SameSite::None);

        self.inner.signed_mut(&self.key).add(cookie);

        self.inner.get("user").unwrap().value()
    }

    fn delta(&self) -> impl Iterator<Item = &Cookie<'static>> {
        self.inner.delta()
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
                return Err(error::ErrorBadRequest("Cannot Access Cookies"));
            };

            for c in cookies.iter() {
                jar.add_original(c.clone());
            }

            Ok(AuthCookieJar {
                inner: jar,
                key: key.clone(),
            })
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
        client: BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(ARGS.auth_url.clone()).unwrap(),
            Some(TokenUrl::new(ARGS.token_url.clone()).unwrap()),
        ),
        reqwest_client: reqwest::Client::new(),
        user_info: ARGS.user_info.clone(),
        singning_key: Key::from(singning_key.as_bytes()),
    }
}

/// Create the auth service under /auth
pub(crate) fn service() -> actix_web::Scope {
    web::scope("/auth")
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
        RedirectUrl::new(format!("{scheme}://{host}/auth/callback/?path={path}")).unwrap(),
    )
}

#[get("/login")]
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
        .add_scope(oauth2::Scope::new("offline_access".to_string()))
        .add_scope(oauth2::Scope::new("profile".to_string()))
        .set_redirect_uri(redirect_url(&path, request))
        .url();

    HttpResponse::Found()
        .append_header((header::LOCATION, auth_url.to_string()))
        .cookie(
            Cookie::build("csrf", csrf_token.secret())
                .same_site(SameSite::None)
                .path("/auth/")
                .finish(),
        )
        .finish()
}

#[get("/logout")]
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
        .append_header((header::LOCATION, "/"))
        .finish()
}

#[get("/callback")]
async fn callback(
    oauth_client: web::Data<OauthClient>,
    query: web::Query<AuthRequest>,
    mut auth_jar: AuthCookieJar,
    request: HttpRequest,
    mut transaction: DatabaseTransaction<'_>,
) -> domain::Result<impl Responder> {
    let code = AuthorizationCode::new(query.code.clone());
    let state = query.state.clone();

    let Some(csrf_token) = request.cookie("csrf") else {
        return Ok(HttpResponse::Unauthorized().body("missing csrf token"));
    };

    if state != csrf_token.value() {
        return Ok(HttpResponse::Unauthorized().body("wrong csrf state"));
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
        return Err(domain::Error::Message(
            "Could not get token from Provider".to_string(),
        ));
    };

    let access_token = token.access_token().secret();
    let refresh_token = match token.refresh_token() {
        Some(refresh_token) => refresh_token.secret(),
        None => "",
    };

    auth_jar.set_access_token(access_token);
    auth_jar.set_refresh_token(refresh_token);

    let Ok(user) = User::from_token(access_token, &oauth_client).await else {
        return Err(domain::Error::Message(
            "Could not access user info".to_string(),
        ));
    };
    auth_jar.set_user_info(&user);

    let mut response_builder = HttpResponse::Found();
    response_builder.append_header((header::LOCATION, path));

    for cookie in auth_jar.delta() {
        response_builder.cookie(cookie.clone());
    }

    let user_name = format!("{}-{}", ARGS.oauth_source_name.as_str(), user.sub.as_str());
    if transaction
        .person_by_user_name(user_name.as_str())
        .await?
        .is_none()
    {
        info!("creating person '{}'", user_name.as_str());
        transaction
            .create_person(user.name.as_str(), user_name.as_str(), None)
            .await?;
    }

    transaction.commit().await?;

    Ok(response_builder.finish())
}
