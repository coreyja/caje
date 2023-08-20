use std::{collections::HashMap, net::SocketAddr, sync::Mutex};

use axum::{body::Bytes, response::IntoResponse, Router};

use http::{header, method, uri::PathAndQuery, HeaderMap, Method, Request, StatusCode, Uri};
use miette::{IntoDiagnostic, Result};
use reqwest::Response;
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

#[axum_macros::debug_handler]
async fn proxy_request(
    url: Uri,
    host: axum::extract::Host,
    headers: HeaderMap,
    method: http::Method,
    bytes: Bytes,
) -> Result<impl IntoResponse, String> {
    let split = host.0.split(':').collect::<Vec<_>>();
    let host_name = split[0];

    if host_name != PROXY_FROM_DOMAIN {
        return Err(format!(
            "We only proxy requests to the specified domain. Found: {} Expected: {}",
            host_name, PROXY_FROM_DOMAIN
        ));
    }

    let path = url
        .path_and_query()
        .cloned()
        .unwrap_or_else(|| PathAndQuery::from_static("/"));

    let url = http::Uri::builder()
        .scheme("http")
        .authority(PROXY_ORIGIN_DOMAIN);
    let url = url
        .path_and_query(path.clone())
        .build()
        .map_err(|_| "Could not build url")?;

    let response = get_potentially_cached_response(method, url, headers, bytes).await?;

    Ok((
        response.status(),
        response.headers().clone(),
        response.into_body(),
    ))
}

type CacheKey = (Method, Uri);

lazy_static::lazy_static! {
    static ref CACHE: Mutex<HashMap<CacheKey, http::Response<Bytes>>> = Mutex::new(HashMap::new());
}

#[tracing::instrument(skip(body))]
async fn get_potentially_cached_response(
    method: Method,
    url: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Result<http::Response<Bytes>, String> {
    info!("Requesting: {}", url);
    {
        let cache = CACHE.lock().unwrap();
        let cached_response = cache.get(&(method.clone(), url.clone()));

        if let Some(cached) = cached_response {
            let mut response = http::Response::builder().status(cached.status());
            for (key, value) in cached.headers().iter() {
                response = response.header(key, value);
            }
            let response = response
                .body(cached.body().clone())
                .map_err(|_| "Could not build response")?;
            // .headers(response.headers().clone())
            // .body(response.body().clone())
            // .map_err(|_| "Could not build response")?;
            return Ok(response);
        }
    }

    let client = reqwest::Client::new();
    let origin_response = client
        .request(method.clone(), url.to_string())
        .headers(headers)
        .body(body)
        .send()
        .await
        .map_err(|_| "Request failed")?;
    let origin_status = origin_response.status();
    let origin_headers = origin_response.headers().clone();
    let origin_bytes = origin_response
        .bytes()
        .await
        .map_err(|_| "Could not get bytes from body")?;

    // let mut response = http::Response::builder().status(origin_response.status());

    // for (key, value) in origin_response.headers().iter() {
    //     response = response.header(key, value);
    // }
    // let response = response
    //     .body(origin_bytes)
    //     .map_err(|_| "Could not build response")?;

    // {
    //     let mut cache = CACHE.lock().unwrap();
    //     cache.insert((method, url), response.clone());
    // }
    {
        let response_to_cache = http_response_from_reqwest_response(
            &origin_status,
            &origin_headers,
            origin_bytes.clone(),
        )
        .map_err(|_| "Could not build response")?;
        let mut cache = CACHE.lock().unwrap();
        cache.insert((method, url), response_to_cache);
    }

    let response =
        http_response_from_reqwest_response(&origin_status, &origin_headers, origin_bytes)
            .map_err(|_| "Could not build response")?;
    Ok(response)
}

fn http_response_from_reqwest_response(
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
