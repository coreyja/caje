use std::net::SocketAddr;

use axum::{response::IntoResponse, routing::get, Router};

use http::{method, uri::PathAndQuery, HeaderMap, Request};
use miette::{IntoDiagnostic, Result};

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

async fn proxy_request<Body>(
    host: axum::extract::Host,
    headers: HeaderMap,
    method: http::Method,
    request: Request<Body>,
) -> Result<impl IntoResponse, String> {
    let uri = request.uri();

    // let host = request
    //     .headers()
    //     .get("host")
    //     .ok_or("No host header specified")?
    //     .to_str()
    //     .map_err(|_| "Could not parse host header")?;
    let split = host.0.split(':').collect::<Vec<_>>();
    let host_name = split[0];

    if host_name != PROXY_FROM_DOMAIN {
        return Err(format!(
            "We only proxy requests to the specified domain. Found: {} Expected: {}",
            host_name, PROXY_FROM_DOMAIN
        ));
    }

    let path = uri
        .path_and_query()
        .cloned()
        .unwrap_or_else(|| PathAndQuery::from_static("/"));

    let client = reqwest::Client::new();

    let url = http::Uri::builder()
        .scheme("http")
        .authority(PROXY_ORIGIN_DOMAIN);
    let url = url
        .path_and_query(path.clone())
        .build()
        .map_err(|_| "Could not build url")?;
    let response = client
        .request(method, url.to_string())
        .headers(headers)
        .send()
        .await
        .map_err(|_| "Request failed")?;

    Ok((
        response.status(),
        response.headers().clone(),
        response
            .bytes()
            .await
            .into_diagnostic()
            .map_err(|_| "Could not get bytes from header")?,
    ))
}
