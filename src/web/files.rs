use async_std::{
    io,
    path::{Component, Path, PathBuf},
};

use actix_files::NamedFile;
use actix_http::{StatusCode, header};
use actix_web::{
    HttpRequest, HttpResponse, Responder,
    body::BoxBody,
    dev::HttpServiceFactory,
    get,
    http::header::{CacheControl, CacheDirective},
};

use crate::{CONTENT_DIR, domain::Capability};

use super::auth::User;

pub(crate) fn service() -> impl HttpServiceFactory {
    serve_files
}

#[get("/{filename:.*}")]
async fn serve_files(req: HttpRequest, user: Option<User>) -> HttpResponse<BoxBody> {
    // decide what the user gets to see
    let base_dir = match user {
        Some(user) if user.has_capability(Capability::ViewProtected) => {
            CONTENT_DIR.protected.as_path()
        }
        Some(user) if user.has_capability(Capability::ViewHidden) => {
            CONTENT_DIR.hidden.as_path()
        }
        _ => CONTENT_DIR.public.as_path(),
    };

    let sub_path: PathBuf = req.match_info().query("filename").parse().unwrap();

    // validate that the sub_path doesnt go backwards
    for component in sub_path.components() {
        if matches!(component, Component::ParentDir | Component::Prefix(_)) {
            return err_not_found(base_dir, req).await;
        }
    }

    let path = base_dir.join(sub_path.as_path());
    let actual_path = if path.is_dir().await {
        path.join("index.html")
    } else {
        path
    };

    log::debug!("path resolved to {:?}", actual_path);

    let file = match NamedFile::open_async(actual_path.as_path()).await {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return try_redirect(base_dir, sub_path.as_path(), req).await;
        }
        Err(_) => return err_not_found(base_dir, req).await,
    };

    log::debug!("serving path {:?}", actual_path);

    // configure headers for cache control
    //
    // we enforce html to always be revalidated. we assume all of our assets to be fingerprinted,
    // so those can be cached
    let must_revalidate = *file.content_type() == mime::TEXT_HTML;

    // we dont want to set Last-Modified on responses. since the content will live in the nix store
    // anyway,  the date will always be 1970-01-01 which is kind of unnessecary to set
    //
    // ETag should only be set if we want the browser to revalidate
    if must_revalidate {
        file.use_last_modified(false)
            .customize()
            .insert_header(CacheControl(vec![
                CacheDirective::Private,
                CacheDirective::MustRevalidate,
                CacheDirective::MaxAge(0),
            ]))
            .respond_to(&req)
            .map_into_boxed_body()
    } else {
        file.use_last_modified(false)
            .use_etag(false)
            .customize()
            .insert_header(CacheControl(vec![
                CacheDirective::Extension("immutable".to_string(), None),
                CacheDirective::MaxAge(31_536_000),
            ]))
            .respond_to(&req)
            .map_into_boxed_body()
    }
}

async fn err_not_found(base_dir: &Path, req: HttpRequest) -> HttpResponse<BoxBody> {
    let path = base_dir.join("de").join("404.html");

    NamedFile::open_async(path)
        .await
        .map(|f| {
            f.customize()
                .with_status(StatusCode::NOT_FOUND)
                .respond_to(&req)
                .map_into_boxed_body()
        })
        .unwrap_or_else(|_| HttpResponse::NotFound().body("<h1>Not found</h1>"))
}

async fn try_redirect(base_dir: &Path, path: &Path, req: HttpRequest) -> HttpResponse<BoxBody> {
    let fs_redirect_path = base_dir.join("de").join(path);

    if !fs_redirect_path.exists().await {
        return err_not_found(base_dir, req).await;
    }

    let redirect_path = format!("/de/{}", path.to_string_lossy());

    HttpResponse::PermanentRedirect()
        .insert_header((header::LOCATION, redirect_path))
        .finish()
}
