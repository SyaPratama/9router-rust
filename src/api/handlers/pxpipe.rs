use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

pub async fn status() -> impl IntoResponse {
    Json(json!({
        "status": "running",
        "port": 8787,
        "pid": 12345,
        "uptime": "24h",
        "version": "1.0.1"
    }))
}

pub async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "checks": {
            "database": "ok",
            "memory": "ok",
            "disk": "ok"
        }
    }))
}
