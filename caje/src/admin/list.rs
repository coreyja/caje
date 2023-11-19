use std::time::SystemTime;

use axum::{extract::State, response::IntoResponse};
use cacache::Metadata;
use http::StatusCode;
use maud::html;
use miette::IntoDiagnostic;
use sqlx::SqlitePool;

use crate::{get_policy_from_cache, CACHE_DIR};

use super::auth::DBSession;

pub(crate) async fn route(
    State(db_pool): State<SqlitePool>,
    _: DBSession,
) -> Result<impl IntoResponse, String> {
    let file_system_entries: Result<Vec<Metadata>, _> =
        tokio::task::spawn_blocking(move || cacache::list_sync(CACHE_DIR).collect())
            .await
            .into_diagnostic()
            .map_err(|e| e.to_string())?;
    let file_system_entries = file_system_entries.unwrap_or_default();

    let db_pages = sqlx::query!("SELECT * FROM Pages")
        .fetch_all(&db_pool)
        .await
        .into_diagnostic()
        .map_err(|e| e.to_string())?;

    let db_pages = db_pages
        .into_iter()
        .map(|page| format!("{} {}", page.method, page.url))
        .collect::<Vec<_>>();

    let resp = html! {
        h2 { "File System" }
        ul {
            @for entry in file_system_entries {
                li { (entry.key) " TTL Seconds: " (get_policy_from_cache(&entry.key).await.unwrap().0.time_to_live(SystemTime::now()).as_secs()) }
            }
        }

        h2 { "Database" }
        ul {
            @for entry in db_pages {
                li { (entry) }
            }
        }
    };

    Ok((StatusCode::OK, resp))
}
