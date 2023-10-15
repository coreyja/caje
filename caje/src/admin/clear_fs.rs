use axum::response::{IntoResponse, Redirect};
use miette::IntoDiagnostic;

use crate::CACHE_DIR;

pub async fn route() -> Result<impl IntoResponse, String> {
    cacache::clear(CACHE_DIR)
        .await
        .into_diagnostic()
        .map_err(|e| e.to_string())?;

    Ok(Redirect::to("/_caje/list"))
}
