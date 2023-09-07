use std::{fs::OpenOptions, net::SocketAddr, time::SystemTime};

use axum::{
    body::{Body, Bytes},
    extract::{Host, State},
    response::IntoResponse,
    RequestExt, Router,
};

use cacache::Metadata;
use http::{uri::PathAndQuery, HeaderMap, Method, Request, Response, StatusCode, Uri, Version};
use http_cache_semantics::{BeforeRequest, CachePolicy};
use miette::{miette, Context, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::info;

const PROXY_FROM_DOMAIN: &str = "slow.coreyja.com";
const PROXY_ORIGIN_DOMAIN: &str = "slow-server.fly.dev";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url: String = std::env::var("DATABASE_URL").or_else(|_| -> Result<String> {
        let path = std::env::var("DATABASE_PATH");

        Ok(if let Ok(p) = &path {
            OpenOptions::new()
                .write(true)
                .create(true)
                .open(p)
                .into_diagnostic()?;

            format!("sqlite:{}", p)
        } else {
            "sqlite::memory:".to_string()
        })
    })?;
    let db_pool = sqlx::sqlite::SqlitePool::connect(&database_url)
        .await
        .into_diagnostic()?;
    sqlx::migrate!().run(&db_pool).await.into_diagnostic()?;

    let app = Router::new()
        .route("/_caje/list", axum::routing::get(admin_list_entries))
        .fallback(proxy_request)
        .with_state(db_pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .into_diagnostic()?;

    Ok(())
}

async fn admin_list_entries() -> Result<impl IntoResponse, String> {
    let entries: Result<Vec<Metadata>, _> =
        tokio::task::spawn_blocking(move || cacache::list_sync(CACHE_DIR).collect())
            .await
            .into_diagnostic()
            .map_err(|e| e.to_string())?;
    let entries = entries.into_diagnostic().map_err(|e| e.to_string())?;

    let entries = entries
        .into_iter()
        .map(|entry| entry.key)
        .collect::<Vec<_>>();

    Ok((StatusCode::OK, entries.join("\n")))
}

// #[axum_macros::debug_handler]
async fn proxy_request(
    State(db_pool): State<SqlitePool>,
    mut request: Request<Body>,
) -> Result<impl IntoResponse, String> {
    let host: Host = request
        .extract_parts()
        .await
        .map_err(|_| "Could not extract host")?;
    let split = host.0.split(':').collect::<Vec<_>>();
    let host_name = split[0];

    if host_name != PROXY_FROM_DOMAIN {
        return Err(format!(
            "We only proxy requests to the specified domain. Found: {} Expected: {}",
            host_name, PROXY_FROM_DOMAIN
        ));
    }

    let response = get_potentially_cached_response(request, db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok((
        response.status(),
        response.headers().clone(),
        response.into_body(),
    ))
}

const CACHE_DIR: &str = "./tmp/cache";

#[derive(Deserialize, Serialize)]
struct InnerCachedRequest {
    #[serde(with = "http_serde::method")]
    pub method: Method,

    #[serde(with = "http_serde::uri")]
    pub uri: Uri,

    #[serde(with = "http_serde::version")]
    pub version: Version,

    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,

    body: Vec<u8>,
}

#[derive(Deserialize, Serialize, Clone)]
struct InnerCachedResponse {
    #[serde(with = "http_serde::status_code")]
    pub status_code: StatusCode,

    #[serde(with = "http_serde::version")]
    pub version: Version,

    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,

    body: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
struct CachedResponse {
    request: InnerCachedRequest,
    response: InnerCachedResponse,
    cached_at: SystemTime,
}

#[tracing::instrument(skip_all)]
async fn get_potentially_cached_response(
    request: Request<Body>,
    db_pool: SqlitePool,
) -> Result<http::Response<Bytes>> {
    let method = request.method().clone();
    let url = request.uri().clone();
    info!("Requesting: {}", url);

    {
        // let cache = CACHE.lock().unwrap();
        // let cached_response = cache.get(&(method.clone(), url.clone()));
        let cache_key = format!("{}@{}", method, url);
        let cached_response = cacache::read(CACHE_DIR, cache_key)
            .await
            .context("Could not write to cache");

        if let Ok(cached) = cached_response {
            let cached = postcard::from_bytes::<CachedResponse>(&cached)
                .map_err(|_| miette!("Could not deserialize cached response"))?;

            let response = http_response_from_parts(cached.response)
                .map_err(|_| miette!("Could not build response"))?;

            let request = http_request_from_parts(cached.request)
                .map_err(|_| miette!("Could not build request"))?;

            let policy =
                CachePolicy::new_options(&request, &response, cached.cached_at, Default::default());
            dbg!(&policy);
            let can_cache = policy.before_request(&request, SystemTime::now());

            match can_cache {
                BeforeRequest::Fresh(_) => {
                    info!("Cache hit for: {}", url);
                    return Ok(response);
                }
                BeforeRequest::Stale { matches, request } => {
                    info!(matches =? matches, request =? request, "Cache hit for: {} but stale", url);
                }
            };
        }
    }

    let path = url
        .path_and_query()
        .cloned()
        .unwrap_or_else(|| PathAndQuery::from_static("/"));

    let proxy_url = http::Uri::builder()
        .scheme("https")
        .authority(PROXY_ORIGIN_DOMAIN)
        .path_and_query(path.clone())
        .build()
        .map_err(|_| miette!("Could not build url"))?;

    let headers = request.headers().clone();
    let bytes = hyper::body::to_bytes(request.into_body())
        .await
        .map_err(|_| miette!("Could not get bytes from body"))?;
    let client = reqwest::Client::new();
    let origin_response = client
        .request(method.clone(), proxy_url.to_string())
        .headers(headers.clone())
        .body(bytes.clone())
        .send()
        .await
        .map_err(|_| miette!("Request failed"))?;

    let origin_status = origin_response.status();
    let origin_headers = origin_response.headers().clone();
    let origin_version = origin_response.version();
    let origin_bytes = origin_response
        .bytes()
        .await
        .map_err(|_| miette!("Could not get bytes from body"))?;

    let parts = InnerCachedResponse {
        status_code: origin_status,
        headers: origin_headers.clone(),
        body: origin_bytes.into(),
        version: origin_version,
    };
    let response_to_cache =
        http_response_from_parts(parts.clone()).map_err(|_| miette!("Could not build response"))?;
    let mut request_to_cache = Request::builder().method(method.clone()).uri(url.clone());
    for (key, value) in headers {
        if let Some(key) = key {
            request_to_cache = request_to_cache.header(key, value);
        }
    }

    let request_to_cache = request_to_cache
        .body(bytes)
        .map_err(|_| miette!("Could not build request"))?;

    let policy = CachePolicy::new(&request_to_cache, &response_to_cache);
    if policy.is_storable() && !policy.time_to_live(SystemTime::now()).is_zero() {
        let response_to_cache = CachedResponse {
            request: request_to_cache.into_inner_cached_request()?,
            response: response_to_cache.into_inner_cached_response()?,
            cached_at: SystemTime::now(),
        };

        let cache_key = format!("{}@{}", method, url);
        cacache::write(
            CACHE_DIR,
            cache_key,
            postcard::to_allocvec(&response_to_cache).into_diagnostic()?,
        )
        .await
        .context("Could not write to cache")?;
        let method = method.to_string();
        let url = url.to_string();

        sqlx::query!("INSERT INTO Pages (method, url) VALUES (?, ?)", method, url)
            .execute(&db_pool)
            .await
            .into_diagnostic()?;
    }

    let response =
        http_response_from_parts(parts).map_err(|_| miette::miette!("Could not build response"))?;

    Ok(response)
}

fn http_response_from_parts(parts: InnerCachedResponse) -> Result<http::Response<Bytes>> {
    let InnerCachedResponse {
        status_code,
        headers,
        body,
        version,
    } = parts;

    let mut builder = http::Response::builder()
        .status(status_code)
        .version(version);

    for (key, value) in headers.iter() {
        builder = builder.header(key, value);
    }

    let body: Bytes = body.into();

    builder.body(body).into_diagnostic()
}

fn http_request_from_parts(parts: InnerCachedRequest) -> Result<http::Request<Bytes>> {
    let InnerCachedRequest {
        method,
        uri,
        version,
        headers,
        body,
    } = parts;

    let mut builder = http::Request::builder()
        .method(method)
        .uri(uri)
        .version(version);

    for (key, value) in headers.iter() {
        builder = builder.header(key, value);
    }

    let body: Bytes = body.into();

    builder.body(body).into_diagnostic()
}

trait IntoInnerCachedRequest {
    fn into_inner_cached_request(self) -> Result<InnerCachedRequest>;
}

impl IntoInnerCachedRequest for Request<Bytes> {
    fn into_inner_cached_request(self) -> Result<InnerCachedRequest> {
        let (parts, body) = self.into_parts();

        Ok(InnerCachedRequest {
            method: parts.method,
            uri: parts.uri,
            version: parts.version,
            headers: parts.headers,
            body: body.to_vec(),
        })
    }
}

trait IntoInnerCachedResponse {
    fn into_inner_cached_response(self) -> Result<InnerCachedResponse>;
}

impl IntoInnerCachedResponse for Response<Bytes> {
    fn into_inner_cached_response(self) -> Result<InnerCachedResponse> {
        let (parts, body) = self.into_parts();

        Ok(InnerCachedResponse {
            status_code: parts.status,
            version: parts.version,
            headers: parts.headers,
            body: body.to_vec(),
        })
    }
}
