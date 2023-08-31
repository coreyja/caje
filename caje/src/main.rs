use std::{collections::HashMap, net::SocketAddr, sync::Mutex, time::SystemTime};

use axum::{
    body::{Body, Bytes},
    extract::Host,
    response::IntoResponse,
    RequestExt, Router,
};

use http::{uri::PathAndQuery, HeaderMap, Method, Request, StatusCode, Uri};
use http_cache_semantics::{BeforeRequest, CachePolicy};
use miette::{IntoDiagnostic, Result};
use tracing::info;

const PROXY_FROM_DOMAIN: &str = "slow.coreyja.test";
const PROXY_ORIGIN_DOMAIN: &str = "localhost:3000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new().fallback(proxy_request);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .into_diagnostic()?;

    Ok(())
}

// #[axum_macros::debug_handler]
async fn proxy_request(mut request: Request<Body>) -> Result<impl IntoResponse, String> {
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

    let response = get_potentially_cached_response(request).await?;

    Ok((
        response.status(),
        response.headers().clone(),
        response.into_body(),
    ))
}

type CacheKey = (Method, Uri);

lazy_static::lazy_static! {
    static ref CACHE: Mutex<HashMap<CacheKey, CachedResponse>> = Mutex::new(HashMap::new());
}

struct CachedResponse {
    request: Request<Bytes>,
    response: http::Response<Bytes>,
    cached_at: SystemTime,
}

#[tracing::instrument(skip_all)]
async fn get_potentially_cached_response(
    request: Request<Body>,
) -> Result<http::Response<Bytes>, String> {
    let method = request.method().clone();
    let url = request.uri().clone();
    info!("Requesting: {}", url);

    {
        let cache = CACHE.lock().unwrap();
        let cached_response = cache.get(&(method.clone(), url.clone()));

        if let Some(cached) = cached_response {
            let response = http_response_from_parts(
                &cached.response.status(),
                cached.response.headers(),
                cached.response.body().clone(),
            )
            .map_err(|_| "Could not build response")?;

            let policy = CachePolicy::new_options(
                &cached.request,
                &response,
                cached.cached_at,
                Default::default(),
            );
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
        .scheme("http")
        .authority(PROXY_ORIGIN_DOMAIN)
        .path_and_query(path.clone())
        .build()
        .map_err(|_| "Could not build url")?;

    let headers = request.headers().clone();
    let bytes = hyper::body::to_bytes(request.into_body())
        .await
        .map_err(|_| "Could not get bytes from body")?;
    let client = reqwest::Client::new();
    let origin_response = client
        .request(method.clone(), proxy_url.to_string())
        .headers(headers.clone())
        .body(bytes.clone())
        .send()
        .await
        .map_err(|_| "Request failed")?;

    let origin_status = origin_response.status();
    let origin_headers = origin_response.headers().clone();
    let origin_bytes = origin_response
        .bytes()
        .await
        .map_err(|_| "Could not get bytes from body")?;

    {
        let response_to_cache =
            http_response_from_parts(&origin_status, &origin_headers, origin_bytes.clone())
                .map_err(|_| "Could not build response")?;
        let mut request_to_cache = Request::builder().method(method.clone()).uri(url.clone());

        for (key, value) in headers {
            if let Some(key) = key {
                request_to_cache = request_to_cache.header(key, value);
            }
        }

        let request_to_cache = request_to_cache
            .body(bytes)
            .map_err(|_| "Could not build request")?;
        let response_to_cache = CachedResponse {
            request: request_to_cache,
            response: response_to_cache,
            cached_at: SystemTime::now(),
        };
        let mut cache = CACHE.lock().unwrap();
        cache.insert((method, url.clone()), response_to_cache);
    }

    let response = http_response_from_parts(&origin_status, &origin_headers, origin_bytes)
        .map_err(|_| "Could not build response")?;
    Ok(response)
}

fn http_response_from_parts(
    status: &StatusCode,
    headers: &HeaderMap,
    body: Bytes,
) -> Result<http::Response<Bytes>> {
    let mut builder = http::Response::builder().status(status);

    for (key, value) in headers.iter() {
        builder = builder.header(key, value);
    }

    builder.body(body).into_diagnostic()
}
