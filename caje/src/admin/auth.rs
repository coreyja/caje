use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, State},
    response::{IntoResponse, Redirect},
    RequestPartsExt,
};
use base64::Engine;
use http::StatusCode;
use maud::html;
use sqlx::{query, query_as, Sqlite, SqlitePool};
use tower_cookies::{Cookie, Cookies, Key};

use crate::AppState;

pub(crate) async fn get(State(app_state): State<AppState>, cookies: Cookies) -> impl IntoResponse {
    let private = cookies.private(&app_state.cookie_key.0);

    let session_cookie = private.get("session_id");
    let session_id = session_cookie.map(|cookie| cookie.value().to_string());
    let db_session = DBSession::fetch_optional(&app_state.db_pool, session_id).await;

    html! {
      @if let Some(s) = db_session {
        p { "Session Id: " (s.id) }
      }

      form method="post" action="/_caje/auth" {
        input type="submit" value="Login";
      }
    }
}

pub(crate) struct DBSession {
    id: i64,
    session_id: String,
}

impl DBSession {
    async fn fetch(db_pool: &SqlitePool, session_id: String) -> Option<DBSession> {
        query_as!(
            DBSession,
            "SELECT * FROM sessions WHERE session_id = $1",
            session_id
        )
        .fetch_optional(db_pool)
        .await
        .unwrap()
    }

    async fn fetch_optional(db_pool: &SqlitePool, session_id: Option<String>) -> Option<DBSession> {
        Self::fetch(db_pool, session_id?).await
    }
}

#[async_trait]
impl FromRequestParts<AppState> for DBSession {
    type Rejection = axum::response::Redirect;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookies = Cookies::from_request_parts(parts, state).await.unwrap();

        let private = cookies.private(&state.cookie_key.0);

        let session_cookie = private.get("session_id");

        let Some(session_cookie) = session_cookie else {
            Err(Redirect::temporary("/_caje/auth"))?
        };
        let session_id = session_cookie.value().to_string();

        Ok(DBSession::fetch(&state.db_pool, session_id).await.unwrap())
    }
}

pub(crate) async fn post(State(state): State<AppState>, cookies: Cookies) -> impl IntoResponse {
    let private = cookies.private(&state.cookie_key.0);

    let mut session_cookie = private.get("session_id");

    if let Some(s) = &session_cookie {
        let session = DBSession::fetch(&state.db_pool, s.value().to_string()).await;

        // If you have a cookie but the DB doesn't know your session
        // Treat it like the cookie doesn't exist
        if session.is_none() {
            session_cookie = None;
        }
    }

    if session_cookie.is_none() {
        let session_id = uuid::Uuid::new_v4().to_string();
        query_as!(
            DBSession,
            "INSERT INTO sessions (session_id) VALUES ($1) returning *",
            session_id
        )
        .fetch_one(&state.db_pool)
        .await
        .unwrap();

        let session_cookie = Cookie::build("session_id", session_id)
            .path("/")
            .http_only(true)
            .secure(true)
            .finish();
        private.add(session_cookie.clone());
    };

    Redirect::to("/_caje/list")
}
