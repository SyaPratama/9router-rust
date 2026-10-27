use axum::{routing::get, Router};
use crate::api::middleware::AppState;
use crate::web::{serve_asset, serve_page};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/landing", get(serve_page))
        .route("/login", get(serve_page))
        .route("/callback", get(serve_page))
        .route("/dashboard/settings/pricing", get(serve_page))
        .route("/site/assets/{*path}", get(serve_asset))
}
