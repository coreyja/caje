use axum::{extract::State, response::IntoResponse};
use base64::Engine;
use maud::html;
use sqlx::{query, query_as};
use tower_cookies::{Cookie, Cookies, Key};

use crate::AppState;

pub(crate) async fn get(State(app_state): State<AppState>, cookies: Cookies) -> impl IntoResponse {
    let private = cookies.private(&app_state.cookie_key.0);

    let session_cookie = private.get("session_id");

    let cookie_display = format!("{:?}", session_cookie);

    html! {
      p { "Session Id: " (cookie_display) }

      form method="post" action="/_caje/auth" {
        input type="submit" value="Login";
      }
    }
}

struct DBSession {
    id: i64,
    session_id: String,
}

pub(crate) async fn post(State(app_state): State<AppState>, cookies: Cookies) -> impl IntoResponse {
    let private = cookies.private(&app_state.cookie_key.0);

    let session_cookie = private.get("session_id");

    let db_session = if let Some(session_cookie) = session_cookie {
        let session_id = session_cookie.value().to_string();

        query_as!(
            DBSession,
            "SELECT * FROM sessions WHERE session_id = $1",
            session_id
        )
        .fetch_optional(&app_state.db_pool)
        .await
        .unwrap()
        .unwrap()
    } else {
        let session_id = uuid::Uuid::new_v4().to_string();
        let db_session = query_as!(
            DBSession,
            "INSERT INTO sessions (session_id) VALUES ($1) returning *",
            session_id
        )
        .fetch_one(&app_state.db_pool)
        .await
        .unwrap();
        let session_cookie = Cookie::build("session_id", session_id)
            .path("/")
            .http_only(true)
            .secure(true)
            .finish();
        private.add(session_cookie.clone());

        db_session
    };

    html! {
      p { "Session Id: " (db_session.session_id) }

      form method="post" action="/_caje/auth" {
        input type="submit" value="Login";
      }
    }
}
