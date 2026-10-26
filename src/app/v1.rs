use axum::{
    routing::{get, post},
    Router,
};

use crate::api::handlers;
use crate::api::middleware::AppState;

pub fn routes() -> Router<AppState> {
    let routes = Router::new()
        .route(
            "/chat/completions",
            post(handlers::chat::handle_chat_completions),
        )
        .route("/messages", post(handlers::claude::handle_claude_messages))
        .route("/responses", post(handlers::responses::handle_responses))
        .route("/models", get(handlers::models::list_models))
        .route("/models/{model}", get(handlers::models::get_model))
        .route("/compress", post(handlers::headroom::handle_compress));

    Router::new().nest("/v1", routes)
}
