use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::api::middleware::{has_valid_auth_session, session_token, AppState};
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
    let require_login = state.engine.pool.get().ok()
        .and_then(|conn| SettingsRepo::get(&conn).ok())
        .map(|settings| settings.require_login)
        .unwrap_or(true);
    let has_password = std::env::var("INITIAL_PASSWORD").is_ok_and(|password| valid_initial_password(&password));

    Json(json!({
        "authenticated": !require_login || has_valid_auth_session(&state, &headers),
        "authDisabled": !require_login,
        "requireLogin": require_login,
        "authMode": "password",
        "displayName": "Password user",
        "hasPassword": has_password,
        "loginMethod": "Password",
        "oidcConfigured": false,
        "oidcLogin": false,
        "samlConfigured": false,
        "samlLogin": false
    }))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginPayload>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let expected = std::env::var("INITIAL_PASSWORD")
        .ok()
        .filter(|password| valid_initial_password(password))
        .ok_or_else(|| (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "success": false, "error": "INITIAL_PASSWORD must be set to a non-default value" })),
        ))?;
    if payload.password.as_deref() != Some(expected.as_str()) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "success": false, "error": "Invalid password" })),
        ));
    }

    let token = uuid::Uuid::new_v4().simple().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    state.auth_sessions.lock()
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, Json(json!({ "success": false, "error": "Session storage unavailable" }))))?
        .insert(token.clone(), expires_at);

    let secure = headers.get("x-forwarded-proto").and_then(|value| value.to_str().ok()) == Some("https");
    let cookie = format!(
        "auth_token={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000{}",
        if secure { "; Secure" } else { "" },
    );
    let mut response = Json(json!({ "success": true, "mustChangePassword": false })).into_response();
    response.headers_mut().insert(header::SET_COOKIE, header::HeaderValue::from_str(&cookie).unwrap());
    Ok(response)
}

async fn logout(headers: HeaderMap, State(state): State<AppState>) -> Response {
    if let Some(token) = session_token(&headers) {
        if let Ok(mut sessions) = state.auth_sessions.lock() {
            sessions.remove(&token);
        }
    }
    let mut response = Json(json!({ "success": true })).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        header::HeaderValue::from_static("auth_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"),
    );
    response
}
fn valid_initial_password(password: &str) -> bool {
    !password.is_empty() && !matches!(password, "123456" | "Tama12345")
}

#[cfg(test)]
mod tests {
    use super::valid_initial_password;

    #[test]
    fn password_must_be_nonempty_and_not_a_default() {
        assert!(!valid_initial_password(""));
        assert!(!valid_initial_password("123456"));
        assert!(!valid_initial_password("Tama12345"));
        assert!(valid_initial_password("configured-secret"));
    }
}
