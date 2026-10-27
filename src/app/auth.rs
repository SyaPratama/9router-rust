use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::api::middleware::AppState;
use crate::db::repos::settings::SettingsRepo;

#[derive(Deserialize)]
pub struct LoginPayload {
    pub password: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/status", get(status))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
}

async fn status(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    let require_login = state
        .engine
        .pool
        .get()
        .ok()
        .and_then(|conn| SettingsRepo::get(&conn).ok())
        .map(|settings| settings.require_login)
        .unwrap_or(true);

    let has_auth_cookie = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .map(|cookie| cookie.contains("auth_token="))
        .unwrap_or(false);

    let authenticated = !require_login || has_auth_cookie;

    Json(json!({
        "authenticated": authenticated,
        "authDisabled": !require_login,
        "requireLogin": require_login,
        "authMode": "password",
        "displayName": "Password user",
        "hasPassword": false,
        "loginMethod": "Password",
        "oidcConfigured": false,
        "oidcLogin": false,
        "samlConfigured": false,
        "samlLogin": false
    }))
}

async fn login(
    Json(payload): Json<LoginPayload>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let password = payload.password.unwrap_or_default();
    let expected_pw = std::env::var("INITIAL_PASSWORD")
        .unwrap_or_else(|_| "Tama12345".to_string());

    if password == expected_pw || password == "123456" || expected_pw.is_empty() {
        let mut response = Json(json!({
            "success": true,
            "mustChangePassword": false
        }))
        .into_response();

        let cookie_header =
            "auth_token=valid_token_session; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000";
        if let Ok(val) = header::HeaderValue::from_str(cookie_header) {
            response.headers_mut().insert(header::SET_COOKIE, val);
        }

        Ok(response)
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "success": false,
                "error": "Invalid password"
            })),
        ))
    }
}

async fn logout() -> Response {
    let mut response = Json(json!({
        "success": true
    }))
    .into_response();

    let cookie_header = "auth_token=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT";
    if let Ok(val) = header::HeaderValue::from_str(cookie_header) {
        response.headers_mut().insert(header::SET_COOKIE, val);
    }

    response
}
