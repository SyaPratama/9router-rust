use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};
use crate::api::middleware::AppState;

pub async fn handle_compress(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let Some(messages) = payload.get("messages").and_then(|m| m.as_array()) {
        let (compressed_messages, stats) = state.engine.headroom.compress_messages(messages.clone());
        Ok(Json(json!({
            "messages": compressed_messages,
            "stats": stats
        })))
    } else if let Some(content) = payload.get("content").and_then(|c| c.as_str()) {
        let (compressed_text, transforms) = state.engine.headroom.compress_text(content);
        let tokens_before = (content.len() + 3) / 4;
        let tokens_after = (compressed_text.len() + 3) / 4;
        let tokens_saved = if tokens_before > tokens_after { tokens_before - tokens_after } else { 0 };
        let ratio = if tokens_before > 0 { tokens_after as f64 / tokens_before as f64 } else { 1.0 };

        Ok(Json(json!({
            "content": compressed_text,
            "stats": {
                "tokensBefore": tokens_before,
                "tokensAfter": tokens_after,
                "tokensSaved": tokens_saved,
                "compressionRatio": ratio,
                "transformsApplied": transforms
            }
        })))
    } else {
        Err((StatusCode::BAD_REQUEST, "Missing 'messages' or 'content' in request body".to_string()))
    }
}
