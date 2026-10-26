use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;
use crate::api::middleware::AppState;
use crate::routing::create_sse_response;
use crate::translator::ResponsesTranslator;

pub async fn handle_responses(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Response, (StatusCode, String)> {
    let stream = payload.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);
    let chat_req = ResponsesTranslator::responses_to_chat_request(&payload);

    let resp = state.engine.dispatch_chat(chat_req).await?;

    if stream {
        let stream = resp.bytes_stream();
        let sse = create_sse_response(stream);
        Ok(sse.into_response())
    } else {
        let status = resp.status();
        let body_bytes = resp.bytes().await.map_err(|e| {
            (StatusCode::BAD_GATEWAY, format!("Failed reading upstream response: {}", e))
        })?;

        if let Ok(openai_val) = serde_json::from_slice::<Value>(&body_bytes) {
            let responses_val = ResponsesTranslator::chat_completion_to_responses(&openai_val);
            let out_bytes = serde_json::to_vec(&responses_val).unwrap_or_default();
            let response = Response::builder()
                .status(status)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(out_bytes))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(response)
        } else {
            let response = Response::builder()
                .status(status)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(body_bytes))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(response)
        }
    }
}
