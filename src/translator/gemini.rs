use super::types::ChatRequest;
use serde_json::{json, Value};

pub struct GeminiTranslator;

impl GeminiTranslator {
    pub fn to_gemini_payload(req: &ChatRequest) -> Value {
        let mut contents = Vec::new();

        for msg in &req.messages {
            let role = if msg.role == "assistant" { "model" } else { "user" };
            let mut parts = Vec::new();

            if let Some(c) = &msg.content {
                if let Some(s) = c.as_str() {
                    parts.push(json!({ "text": s }));
                } else if let Some(arr) = c.as_array() {
                    for item in arr {
                        if let Some(txt) = item.get("text").and_then(|t| t.as_str()) {
                            parts.push(json!({ "text": txt }));
                        }
                    }
                }
            }

            if parts.is_empty() {
                parts.push(json!({ "text": "" }));
            }

            contents.push(json!({
                "role": role,
                "parts": parts
            }));
        }

        json!({
            "contents": contents
        })
    }

    pub fn gemini_to_openai_response(val: &Value) -> Value {
        let text = val.get("candidates")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|cand| cand.get("content"))
            .and_then(|cont| cont.get("parts"))
            .and_then(|parts| parts.as_array())
            .and_then(|arr| arr.first())
            .and_then(|part| part.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        json!({
            "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            "object": "chat.completion",
            "created": chrono::Utc::now().timestamp(),
            "model": "gemini",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": text
                },
                "finish_reason": "stop"
            }]
        })
    }
}
