use axum::{extract::State, response::IntoResponse};
use base64::Engine;
use maud::html;
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

pub(crate) async fn post(State(app_state): State<AppState>, cookies: Cookies) -> impl IntoResponse {
    let private = cookies.private(&app_state.cookie_key.0);

    let session_cookie = private.get("session_id");

    let session_cookie: Cookie = if let Some(session_cookie) = session_cookie {
        session_cookie
    } else {
        let session_cookie = Cookie::build("session_id", "1234567890")
            .path("/")
            .http_only(true)
            .secure(true)
            .finish();

        session_cookie
    };

    private.add(session_cookie.clone());

    let cookie_display = format!("{:?}", session_cookie);

    html! {
      p { "Session Id: " (cookie_display) }

      form method="post" action="/_caje/auth" {
        input type="submit" value="Login";
      }
    }
}
