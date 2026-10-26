pub mod middleware;
pub mod handlers;

use axum::routing::{delete, get, post};
use axum::{middleware as axum_mw, Router};
use std::sync::Arc;
use crate::dashboard::serve_dashboard;
use crate::routing::RoutingEngine;
use middleware::{security_middleware, AppState};

pub fn build_router(engine: Arc<RoutingEngine>) -> Router {
    let state = AppState { engine };

    let v1_routes = Router::new()
        .route("/chat/completions", post(handlers::chat::handle_chat_completions))
        .route("/messages", post(handlers::claude::handle_claude_messages))
        .route("/responses", post(handlers::responses::handle_responses))
        .route("/models", get(handlers::models::list_models))
        .route("/models/{model}", get(handlers::models::get_model))
        .route("/compress", post(handlers::headroom::handle_compress));

    let api_routes = Router::new()
        .route("/health", get(handlers::admin::get_health))
        .route("/settings", get(handlers::admin::get_settings).post(handlers::admin::update_settings))
        .route("/providers", get(handlers::admin::list_providers).post(handlers::admin::save_provider))
        .route("/providers/{id}", delete(handlers::admin::delete_provider))
        .route("/combos", get(handlers::admin::list_combos).post(handlers::admin::save_combo))
        .route("/combos/{id}", delete(handlers::admin::delete_combo))
        .route("/keys", get(handlers::admin::list_keys).post(handlers::admin::save_key))
        .route("/keys/{id}", delete(handlers::admin::delete_key))
        .route("/usage/stats", get(handlers::admin::get_usage_stats));

    Router::new()
        .nest("/v1", v1_routes)
        .nest("/api", api_routes)
        .route("/", get(serve_dashboard))
        .route("/dashboard", get(serve_dashboard))
        .route("/dashboard/{*path}", get(serve_dashboard))
        .layer(axum_mw::from_fn_with_state(state.clone(), security_middleware))
        .with_state(state)
}
