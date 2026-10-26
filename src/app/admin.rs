use axum::{routing::{delete, get}, Router};

use crate::api::handlers;
use crate::api::middleware::AppState;

pub fn routes() -> Router<AppState> {
    let routes = Router::new()
        .route("/health", get(handlers::admin::get_health))
        .route("/summary", get(handlers::admin::get_summary))
        .route(
            "/settings",
            get(handlers::admin::get_settings).post(handlers::admin::update_settings),
        )
        .route(
            "/providers",
            get(handlers::admin::list_providers).post(handlers::admin::save_provider),
        )
        .route("/providers/{id}", delete(handlers::admin::delete_provider))
        .route(
            "/combos",
            get(handlers::admin::list_combos).post(handlers::admin::save_combo),
        )
        .route("/combos/{id}", delete(handlers::admin::delete_combo))
        .route(
            "/keys",
            get(handlers::admin::list_keys).post(handlers::admin::save_key),
        )
        .route("/keys/{id}", delete(handlers::admin::delete_key))
        .route("/usage/stats", get(handlers::admin::get_usage_stats));

    Router::new().nest("/api", routes)
}
