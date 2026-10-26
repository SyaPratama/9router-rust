use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use serde_json::json;

use crate::api::middleware::AppState;
use crate::db::repos::settings::SettingsRepo;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/auth/status", get(status))
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let require_login = state
        .engine
        .pool
        .get()
        .ok()
        .and_then(|conn| SettingsRepo::get(&conn).ok())
        .map(|settings| settings.require_api_key)
        .unwrap_or(false);

    Json(json!({
        "authenticated": true,
        "requireLogin": require_login,
        "mode": "local"
    }))
}
