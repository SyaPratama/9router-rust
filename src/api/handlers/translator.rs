use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::time::Instant;
use crate::api::middleware::AppState;
use crate::translator::claude::ClaudeTranslator;
use crate::translator::gemini::GeminiTranslator;
use crate::translator::openai::OpenAiTranslator;
use crate::translator::responses::ResponsesTranslator;
use crate::translator::types::ChatRequest;

#[derive(Debug, Deserialize)]
pub struct TranslatorRunRequest {
    pub source_format: String,
    pub target_format: String,
    pub payload: serde_json::Value,
    pub dry_run: Option<bool>,
}

pub async fn run_translator(
    State(_state): State<AppState>,
    Json(req): Json<TranslatorRunRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let start = Instant::now();

    // 1. Normalize source payload into internal ChatRequest
    let chat_req: ChatRequest = match req.source_format.to_lowercase().as_str() {
        "openai" => serde_json::from_value(req.payload.clone()).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to parse OpenAI payload: {}", e),
            )
        })?,
        "responses" => ResponsesTranslator::responses_to_chat_request(&req.payload),
        "claude" | "anthropic" => {
            let model = req
                .payload
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or("claude-3-5-sonnet")
                .to_string();

            let mut messages = Vec::new();

            // Extract system prompt if present
            if let Some(sys) = req.payload.get("system").and_then(|s| s.as_str()) {
                messages.push(crate::translator::types::ChatMessage {
                    role: "system".to_string(),
                    content: Some(serde_json::Value::String(sys.to_string())),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }

            if let Some(msgs_arr) = req.payload.get("messages").and_then(|m| m.as_array()) {
                for m in msgs_arr {
                    let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("user").to_string();
                    let content = m.get("content").cloned();
                    messages.push(crate::translator::types::ChatMessage {
                        role,
                        content,
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
            }

            ChatRequest {
                model,
                messages,
                temperature: req.payload.get("temperature").and_then(|t| t.as_f64()),
                stream: req.payload.get("stream").and_then(|s| s.as_bool()).unwrap_or(false),
                max_tokens: req.payload.get("max_tokens").and_then(|m| m.as_i64()),
                tools: None,
            }
        }
        _ => serde_json::from_value(req.payload.clone()).unwrap_or_else(|_| ChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![],
            temperature: None,
            stream: false,
            max_tokens: None,
            tools: None,
        }),
    };

    // 2. Convert internal ChatRequest to target format
    let output: serde_json::Value = match req.target_format.to_lowercase().as_str() {
        "claude" | "anthropic" => ClaudeTranslator::to_anthropic_payload(&chat_req),
        "gemini" => GeminiTranslator::to_gemini_payload(&chat_req),
        "responses" => {
            let input_text = chat_req
                .messages
                .iter()
                .filter_map(|m| m.content.as_ref().and_then(|c| c.as_str()))
                .collect::<Vec<&str>>()
                .join("\n");

            json!({
                "model": chat_req.model,
                "input": input_text,
                "stream": chat_req.stream,
                "temperature": chat_req.temperature,
                "max_output_tokens": chat_req.max_tokens
            })
        }
        _ => OpenAiTranslator::to_openai_payload(&chat_req),
    };

    let elapsed = start.elapsed().as_millis() as u64;
    let tokens_estimated = serde_json::to_string(&output).unwrap_or_default().len() / 4;

    Ok(Json(json!({
        "source_format": req.source_format,
        "target_format": req.target_format,
        "valid": true,
        "output": output,
        "stats": {
            "tokens_estimated": tokens_estimated,
            "translation_time_ms": elapsed
        }
    })))
}
