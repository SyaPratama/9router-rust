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
        // Providers Catalog & Management
        .route("/providers/catalog", get(handlers::admin::get_provider_catalog))
        .route("/skills", get(handlers::admin::get_skills_catalog))
        .route("/skills/search", get(handlers::admin::search_github_skills))
        .route("/providers/test-batch", post(handlers::admin::test_batch_providers))
        .route("/providers/validate", post(handlers::admin::validate_provider_config))
        .route(
            "/providers",
            get(handlers::admin::list_providers).post(handlers::admin::save_provider),
        )
        .route(
            "/providers/{id}",
            get(handlers::admin::get_provider_or_connection).delete(handlers::admin::delete_provider),
        )
        .route("/providers/{id}/test", post(handlers::admin::test_provider))
        .route(
            "/providers/{id}/models",
            get(handlers::admin::get_provider_models).post(handlers::admin::add_provider_model),
        )
        .route(
            "/providers/{id}/models/{modelId}",
            delete(handlers::admin::delete_provider_model),
        )
        // Combos
        .route("/combos/presets", get(handlers::admin::get_combo_presets))
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
        // Usage Stats & Details
        .route("/usage/stats", get(handlers::admin::get_usage_stats))
        .route("/usage/history", get(handlers::admin::get_usage_history))
        .route("/usage/logs", get(handlers::admin::get_usage_logs))
        .route("/usage/chart", get(handlers::admin::get_usage_chart))
        // Token Saver & Headroom
        .route("/headroom/status", get(handlers::admin::get_headroom_status))
        .route("/headroom/toggle", post(handlers::admin::toggle_headroom))
        .route("/headroom/extras", post(handlers::admin::toggle_headroom))
        // CLI Tools
        .route(
            "/cli-tools/all-statuses",
            get(handlers::admin::get_cli_tool_statuses),
        )
        // Tunnel
        .route("/tunnel/status", get(handlers::admin::get_tunnel_status))
        // Live Console Log SSE
        .route(
            "/console-log/stream",
            get(handlers::console_log::stream_console_logs),
        )
        // Translator Runner & Endpoints
        .route("/translator/run", post(handlers::translator::run_translator))
        .route("/translator/load", get(handlers::admin::get_translator_samples))
        .route("/translator/translate", post(handlers::translator::run_translator))
        .route("/translator/send", post(handlers::translator::run_translator))
        .route("/translator/save", post(handlers::admin::save_translator_sample))
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
