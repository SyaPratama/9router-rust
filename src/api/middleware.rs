use axum::extract::State;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;
use crate::db::repos::keys::ApiKeysRepo;
use crate::db::repos::settings::SettingsRepo;
use crate::routing::RoutingEngine;


#[derive(Clone)]
pub struct OAuthSession {
    pub provider: String,
    pub redirect_uri: String,
    pub browser_binding: String,
    pub verifier: Option<String>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<RoutingEngine>,
    pub oauth_sessions: Arc<std::sync::Mutex<std::collections::HashMap<String, OAuthSession>>>,
    pub auth_sessions: Arc<std::sync::Mutex<std::collections::HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

pub fn session_token(headers: &HeaderMap) -> Option<String> {
    headers.get(axum::http::header::COOKIE)?.to_str().ok()?.split(';').find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == "auth_token" && !value.is_empty()).then(|| value.to_string())
    })
}

pub fn has_valid_auth_session(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(token) = session_token(headers) else { return false };
    let Ok(mut sessions) = state.auth_sessions.lock() else { return false };
    let now = chrono::Utc::now();
    sessions.retain(|_, expires_at| &*expires_at > &now);
    sessions.contains_key(&token)
}

pub async fn security_middleware(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {

    let headers = request.headers();


    let path = request.uri().path();
    let is_v1 = path.starts_with("/v1/") || path.starts_with("/api/v1/");

    if is_v1 {
        let conn = state.engine.pool.get()
            .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "API key authorization unavailable"))?;
        let settings = SettingsRepo::get(&conn)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "API key settings unavailable"))?;
        if settings.require_api_key {
            let token = headers.get("authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer ").map(str::trim))
                .or_else(|| headers.get("x-api-key").and_then(|h| h.to_str().ok()))
                .ok_or((StatusCode::UNAUTHORIZED, "API key required"))?;
            let key = ApiKeysRepo::find_by_key(&conn, token)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "API key lookup failed"))?;
            authorize_api_key(true, Some(token), key.as_ref())?;
        }
    }

    let dashboard_api = path.starts_with("/api/")
        && !path.starts_with("/api/auth/")
        && !path.starts_with("/api/v1/")
        && path != "/api/health"
        && path != "/api/version";
    if dashboard_api {
        let conn = state.engine.pool.get()
            .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Dashboard authorization unavailable"))?;
        let settings = SettingsRepo::get(&conn)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Dashboard authorization unavailable"))?;
        if settings.require_login && !has_valid_auth_session(&state, headers) {
            return Err((StatusCode::UNAUTHORIZED, "Dashboard login required"));
        }
    }

    Ok(next.run(request).await)
}

fn authorize_api_key(
    required: bool,
    token: Option<&str>,
    key: Option<&crate::db::repos::keys::ApiKeyItem>,
) -> Result<(), (StatusCode, &'static str)> {
    if !required {
        return Ok(());
    }
    let (Some(token), Some(key)) = (token, key) else {
        return Err((StatusCode::UNAUTHORIZED, "API key invalid or required"));
    };
    if key.key != token {
        return Err((StatusCode::UNAUTHORIZED, "API key invalid or required"));
    }
    if !key.is_active {
        return Err((StatusCode::UNAUTHORIZED, "API key is deactivated"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{authorize_api_key, session_token};
    use axum::http::{HeaderMap, HeaderValue};
    use axum::http::StatusCode;
    use crate::db::repos::keys::ApiKeyItem;

    #[test]
    fn required_key_rejects_missing_unknown_and_inactive_keys() {
        let active = ApiKeyItem {
            id: "id".into(), key: "valid".into(), name: None, machine_id: None,
            is_active: true, created_at: String::new(), access_restricted: false,
            access_allow: None,
        };
        let inactive = ApiKeyItem { is_active: false, ..active.clone() };
        assert_eq!(authorize_api_key(true, None, None), Err((StatusCode::UNAUTHORIZED, "API key invalid or required")));
        assert_eq!(authorize_api_key(true, Some("bad"), None), Err((StatusCode::UNAUTHORIZED, "API key invalid or required")));
        assert_eq!(authorize_api_key(true, Some("wrong"), Some(&active)), Err((StatusCode::UNAUTHORIZED, "API key invalid or required")));
        assert_eq!(authorize_api_key(true, Some("valid"), Some(&inactive)), Err((StatusCode::UNAUTHORIZED, "API key is deactivated")));
        assert_eq!(authorize_api_key(true, Some("valid"), Some(&active)), Ok(()));
        assert_eq!(authorize_api_key(false, None, None), Ok(()));
    }

    #[test]
    fn session_cookie_requires_a_nonempty_exact_cookie_name() {
        let mut headers = HeaderMap::new();
        assert_eq!(session_token(&headers), None);
        headers.insert(axum::http::header::COOKIE, HeaderValue::from_static("other=1; auth_token=abc"));
        assert_eq!(session_token(&headers).as_deref(), Some("abc"));
        headers.insert(axum::http::header::COOKIE, HeaderValue::from_static("auth_token=; auth_token2=spoof"));
        assert_eq!(session_token(&headers), None);
    }
}
