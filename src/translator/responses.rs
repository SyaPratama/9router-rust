use super::types::{ChatMessage, ChatRequest};
use serde_json::{json, Value};

pub struct ResponsesTranslator;

impl ResponsesTranslator {
    pub fn responses_to_chat_request(val: &Value) -> ChatRequest {
        let model = val.get("model").and_then(|m| m.as_str()).unwrap_or("gpt-4o").to_string();
        let stream = val.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);
        let instructions = val.get("instructions").and_then(|i| i.as_str());

        let mut messages = Vec::new();
        if let Some(inst) = instructions {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(json!(inst)),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            });
        }

        if let Some(input) = val.get("input") {
            if let Some(s) = input.as_str() {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: Some(json!(s)),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            } else if let Some(arr) = input.as_array() {
                for item in arr {
                    let role = item.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                    let content = item.get("content").cloned();
                    messages.push(ChatMessage {
                        role: role.to_string(),
                        content,
                        name: item.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()),
                        tool_call_id: item.get("tool_call_id").and_then(|t| t.as_str()).map(|s| s.to_string()),
                        tool_calls: item.get("tool_calls").and_then(|t| t.as_array()).cloned(),
                    });
                }
            }
        }

        ChatRequest {
            model,
            messages,
            stream,
            temperature: val.get("temperature").and_then(|t| t.as_f64()),
            max_tokens: val.get("max_output_tokens").and_then(|t| t.as_i64()),
            tools: val.get("tools").and_then(|t| t.as_array()).cloned(),
        }
    }

    pub fn chat_completion_to_responses(val: &Value) -> Value {
        let content = val.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");

        let id = val.get("id").and_then(|i| i.as_str()).unwrap_or("resp_123");
        let model = val.get("model").and_then(|m| m.as_str()).unwrap_or("gpt-4o");

        json!({
            "id": id,
            "object": "response",
            "created_at": chrono::Utc::now().timestamp(),
            "model": model,
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": content
                }]
            }]
        })
    }
}
