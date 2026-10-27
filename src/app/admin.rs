use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::api::handlers;
use crate::api::middleware::AppState;

pub fn routes() -> Router<AppState> {
    let routes = Router::new()
        // Health & Version
        .route("/health", get(handlers::admin::get_health))
        .route("/version", get(handlers::admin::get_version))
        .route("/summary", get(handlers::admin::get_summary))
        .route("/quota", get(handlers::admin::get_quota))
        // Settings
        .route(
            "/settings",
            get(handlers::admin::get_settings).post(handlers::admin::update_settings),
        )
        // Providers
        .route(
            "/providers",
            get(handlers::admin::list_providers).post(handlers::admin::save_provider),
        )
        .route("/providers/{id}", delete(handlers::admin::delete_provider))
        // Combos
        .route(
            "/combos",
            get(handlers::admin::list_combos).post(handlers::admin::save_combo),
        )
        .route("/combos/{id}", delete(handlers::admin::delete_combo))
        // API Keys
        .route(
            "/keys",
            get(handlers::admin::list_keys).post(handlers::admin::save_key),
        )
        .route("/keys/{id}", delete(handlers::admin::delete_key))
        // Usage Stats
        .route("/usage/stats", get(handlers::admin::get_usage_stats))
        // Live Console Log SSE
        .route(
            "/console-log/stream",
            get(handlers::console_log::stream_console_logs),
        )
        // Translator Runner
        .route("/translator/run", post(handlers::translator::run_translator))
        // Proxy Pools
        .route(
            "/proxy-pools",
            get(handlers::proxy_pools::list_proxy_pools)
                .post(handlers::proxy_pools::save_proxy_pool),
        )
        .route(
            "/proxy-pools/{id}",
            delete(handlers::proxy_pools::delete_proxy_pool),
        )
        .route(
            "/proxy-pools/{id}/test",
            post(handlers::proxy_pools::test_proxy_pool),
        );

    Router::new().nest("/api", routes)
}
