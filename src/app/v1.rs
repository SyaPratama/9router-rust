use axum::{
    routing::{get, post},
    Router,
};

use crate::api::handlers;
use crate::api::middleware::AppState;

pub fn routes() -> Router<AppState> {
    let routes = Router::new()
        // Chat & Messages
        .route(
            "/chat/completions",
            post(handlers::chat::handle_chat_completions),
        )
        .route("/messages", post(handlers::claude::handle_claude_messages))
        .route("/responses", post(handlers::responses::handle_responses))
        // Headroom Compression
        .route("/compress", post(handlers::headroom::handle_compress))
        // Models Discovery
        .route("/models", get(handlers::models::list_models))
        .route("/models/info", get(handlers::models::get_model_info))
        .route("/models/{model_or_kind}", get(handlers::models::get_model_or_kind))
        // Embeddings
        .route("/embeddings", post(handlers::embeddings::handle_embeddings))
        // Images
        .route("/images/generations", post(handlers::image::handle_image_generations))
        // Audio (STT & TTS)
        .route("/audio/transcriptions", post(handlers::audio::handle_transcription))
        .route("/audio/speech", post(handlers::audio::handle_speech))
        .route("/audio/voices", get(handlers::audio::list_voices))
        // Video Generation
        .route("/videos/generations", post(handlers::video::handle_video_generations))
        .route("/videos/edits", post(handlers::video::handle_video_edits))
        .route("/videos/extensions", post(handlers::video::handle_video_extensions))
        .route("/videos/{request_id}", get(handlers::video::get_video_status))
        // Web Search & Web Fetch
        .route("/search", post(handlers::web::handle_search))
        .route("/web/fetch", post(handlers::web::handle_web_fetch));

    Router::new().nest("/v1", routes)
}
