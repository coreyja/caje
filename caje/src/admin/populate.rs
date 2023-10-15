use std::time::SystemTime;

use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use http::{header::HOST, uri::PathAndQuery, Method, Request, Uri};
use http_cache_semantics::CachePolicy;
use hyper::body::Bytes;
use miette::{Context, IntoDiagnostic};
use sqlx::SqlitePool;

use crate::{
    cache_key, get_policy_from_cache, http_response_from_parts, CachedResponse,
    InnerCachedResponse, IntoInnerCachedRequest, IntoInnerCachedResponse, WrappedError, CACHE_DIR,
    PROXY_FROM_DOMAIN, PROXY_ORIGIN_DOMAIN,
};

#[axum_macros::debug_handler]
pub async fn route(State(db_pool): State<SqlitePool>) -> Result<impl IntoResponse, WrappedError> {
    let db_pages = sqlx::query!("SELECT * FROM Pages")
        .fetch_all(&db_pool)
        .await
        .into_diagnostic()?;
    let now = SystemTime::now();

    for page in db_pages {
        let cache_key = cache_key(&page.method, &page.url);
        let policy = get_policy_from_cache(&cache_key).await;

        if policy.is_ok_and(|(p, _)| !p.time_to_live(now).is_zero()) {
            continue;
        }

        let path = page
            .url
            .parse::<Uri>()
            .into_diagnostic()?
            .path_and_query()
            .cloned()
            .unwrap_or_else(|| PathAndQuery::from_static("/"));

        let proxy_url = http::Uri::builder()
            .scheme("https")
            .authority(PROXY_ORIGIN_DOMAIN)
            .path_and_query(path.clone())
            .build()
            .map_err(|_| miette::miette!("Could not build url"))?;

        let client = reqwest::Client::new();
        let method: Method = page.method.clone().parse().into_diagnostic()?;
        let origin_response = client
            .request(method.clone(), proxy_url.to_string())
            .send()
            .await
            .map_err(|_| miette::miette!("Request failed"))?;

        let origin_status = origin_response.status();
        let origin_headers = origin_response.headers().clone();
        let origin_version = origin_response.version();
        let origin_bytes = origin_response
            .bytes()
            .await
            .map_err(|_| miette::miette!("Could not get bytes from body"))?;

        let parts = InnerCachedResponse {
            status_code: origin_status,
            headers: origin_headers.clone(),
            body: origin_bytes.into(),
            version: origin_version,
        };
        let response_to_cache = http_response_from_parts(parts.clone())
            .map_err(|_| miette::miette!("Could not build response"))?;
        let request_to_cache: Request<()> = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, PROXY_FROM_DOMAIN)
            .body(())
            .into_diagnostic()?;

        let policy = CachePolicy::new(&request_to_cache, &response_to_cache);

        if policy.is_storable() && !policy.time_to_live(SystemTime::now()).is_zero() {
            let response_to_cache = CachedResponse {
                request: request_to_cache.into_inner_cached_request()?,
                response: response_to_cache.into_inner_cached_response()?,
                cached_at: SystemTime::now(),
            };

            cacache::write(
                CACHE_DIR,
                cache_key,
                postcard::to_allocvec(&response_to_cache).into_diagnostic()?,
            )
            .await
            .context("Could not write to cache")?;
        }
    }

    Ok(Redirect::to("/_caje/list"))
}
