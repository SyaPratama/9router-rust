use axum::{routing::get, Router};

use crate::dashboard::{serve_asset, serve_dashboard};

pub fn routes() -> Router<crate::api::middleware::AppState> {
    Router::new()
        .route("/dashboard/assets/{*path}", get(serve_asset))
        .route("/dashboard", get(serve_dashboard))
        .route("/dashboard/{*path}", get(serve_dashboard))
}
