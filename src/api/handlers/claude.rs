use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;
use crate::api::middleware::AppState;
use crate::routing::create_sse_response;
use crate::translator::{ChatMessage, ChatRequest};

pub async fn handle_claude_messages(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Response, (StatusCode, String)> {
    let model = payload.get("model").and_then(|m| m.as_str()).unwrap_or("claude-3-5-sonnet").to_string();
    let stream = payload.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    let mut messages = Vec::new();
    if let Some(sys) = payload.get("system").and_then(|s| s.as_str()) {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: Some(serde_json::Value::String(sys.to_string())),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        });
    }

    if let Some(msg_arr) = payload.get("messages").and_then(|m| m.as_array()) {
        for m in msg_arr {
            let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let content = m.get("content").cloned();
            messages.push(ChatMessage {
                role: role.to_string(),
                content,
                name: None,
                tool_call_id: None,
                tool_calls: None,
            });
        }
    }

    let chat_req = ChatRequest {
        model,
        messages,
        stream,
        temperature: payload.get("temperature").and_then(|t| t.as_f64()),
        max_tokens: payload.get("max_tokens").and_then(|t| t.as_i64()),
        tools: payload.get("tools").and_then(|t| t.as_array()).cloned(),
    };

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

        // If upstream was OpenAI, convert to Anthropic response
        if let Ok(openai_val) = serde_json::from_slice::<Value>(&body_bytes) {
            let claude_resp = if openai_val.get("choices").is_some() {
                // OpenAI to Claude response
                let content_str = openai_val.get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("");

                serde_json::json!({
                    "id": format!("msg_{}", uuid::Uuid::new_v4()),
                    "type": "message",
                    "role": "assistant",
                    "content": [{
                        "type": "text",
                        "text": content_str
                    }],
                    "model": openai_val.get("model").and_then(|m| m.as_str()).unwrap_or("claude"),
                    "stop_reason": "end_turn",
                    "usage": {
                        "input_tokens": 10,
                        "output_tokens": 10
                    }
                })
            } else {
                openai_val
            };

            let out_bytes = serde_json::to_vec(&claude_resp).unwrap_or_default();
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
