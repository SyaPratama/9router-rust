use axum::extract::{ConnectInfo, State};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use std::net::SocketAddr;
use std::sync::Arc;
use crate::auth::{generate_peer_token, is_loopback};
use crate::db::repos::keys::ApiKeysRepo;
use crate::db::repos::settings::SettingsRepo;
use crate::routing::RoutingEngine;

#[derive(Clone)]
pub struct OAuthSession {
    pub provider: String,
    pub redirect_uri: String,
    pub verifier: Option<String>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<RoutingEngine>,
    pub oauth_sessions: Arc<std::sync::Mutex<std::collections::HashMap<String, OAuthSession>>>,
}

pub async fn security_middleware(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    let addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0)
        .unwrap_or_else(|| "127.0.0.1:0".parse().unwrap());

    let headers = request.headers();

    // 1. IP extraction & anti-spoofing
    let is_local = is_loopback(&addr);
    let _client_ip = if is_local {
        // Only trust forwarded headers if connection comes from loopback reverse proxy
        headers.get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| addr.ip().to_string())
    } else {
        addr.ip().to_string()
    };

    let _peer_token = generate_peer_token(&addr, "9router-secure-salt");

    // 2. Check if path requires API key
    let path = request.uri().path();
    let is_v1 = path.starts_with("/v1/");

    if is_v1 {
        let auth_header = headers.get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer ").map(|k| k.trim()))
            .or_else(|| headers.get("x-api-key").and_then(|h| h.to_str().ok()));

        if let Some(token) = auth_header {
            if let Ok(conn) = state.engine.pool.get() {
                if let Ok(Some(key_item)) = ApiKeysRepo::find_by_key(&conn, token) {
                    if !key_item.is_active {
                        return Err((StatusCode::UNAUTHORIZED, "API key is deactivated"));
                    }
                }
            }
        } else {
            if let Ok(conn) = state.engine.pool.get() {
                if let Ok(settings) = SettingsRepo::get(&conn) {
                    if settings.require_api_key {
                        return Err((StatusCode::UNAUTHORIZED, "API key required"));
                    }
                }
            }
        }
    }

    Ok(next.run(request).await)
}
