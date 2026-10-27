use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;
use crate::api::middleware::AppState;
use crate::routing::create_sse_response;
use crate::translator::{ChatMessage, ChatRequest};

use crate::api::handlers::chat::record_usage;
fn claude_usage(value: &Value) -> Option<Value> {
    let usage = value.get("usage")?;
    let mut result = serde_json::Map::new();
    for (source, target) in [("prompt_tokens", "input_tokens"), ("completion_tokens", "output_tokens")] {
        if let Some(tokens) = usage.get(source) {
            result.insert(target.to_string(), tokens.clone());
        }
    }
    (!result.is_empty()).then_some(Value::Object(result))
}

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

    let dispatch = state.engine.dispatch_chat(chat_req).await?;
    let provider = dispatch.provider;
    let model = dispatch.model;
    let connection_id = dispatch.connection_id;
    let resp = dispatch.response;

    if stream {
        if resp.status().is_success() {
            record_usage(state.engine.pool.clone(), provider, model, connection_id, "/v1/messages", None, "accepted");
        }
        let sse = create_sse_response(resp.bytes_stream());
        Ok(sse.into_response())
    } else {
        let status = resp.status();
        let body_bytes = resp.bytes().await.map_err(|e| {
            (StatusCode::BAD_GATEWAY, format!("Failed reading upstream response: {}", e))
        })?;
        if status.is_success() {
            record_usage(state.engine.pool.clone(), provider, model, connection_id, "/v1/messages", Some(&body_bytes), "success");
        }

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

                let mut claude_resp = serde_json::json!({
                    "id": format!("msg_{}", uuid::Uuid::new_v4()),
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "text", "text": content_str }],
                    "model": openai_val.get("model").and_then(|m| m.as_str()).unwrap_or("claude"),
                    "stop_reason": "end_turn"
                });
                if let Some(usage) = claude_usage(&openai_val) {
                    claude_resp["usage"] = usage;
                }
                claude_resp
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

#[cfg(test)]
mod tests {
    use super::claude_usage;
    use serde_json::json;

    #[test]
    fn maps_only_reported_token_counts() {
        assert_eq!(claude_usage(&json!({ "usage": { "prompt_tokens": 12, "completion_tokens": 5 } })), Some(json!({ "input_tokens": 12, "output_tokens": 5 })));
        assert_eq!(claude_usage(&json!({ "choices": [] })), None);
    }
}
