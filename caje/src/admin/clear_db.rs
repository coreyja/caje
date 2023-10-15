use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use miette::IntoDiagnostic;
use sqlx::SqlitePool;

pub async fn route(State(db_pool): State<SqlitePool>) -> Result<impl IntoResponse, String> {
    sqlx::query!("DELETE FROM Pages")
        .execute(&db_pool)
        .await
        .into_diagnostic()
        .map_err(|e| e.to_string())?;

    Ok(Redirect::to("/_caje/list"))
}
