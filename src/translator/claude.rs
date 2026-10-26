use super::types::ChatRequest;
use serde_json::{json, Value};

pub struct ClaudeTranslator;

impl ClaudeTranslator {
    pub fn to_anthropic_payload(req: &ChatRequest) -> Value {
        let mut system = String::new();
        let mut messages = Vec::new();

        for msg in &req.messages {
            if msg.role == "system" {
                if let Some(c) = &msg.content {
                    if let Some(s) = c.as_str() {
                        if !system.is_empty() {
                            system.push('\n');
                        }
                        system.push_str(s);
                    }
                }
            } else if msg.role == "tool" {
                // OpenAI tool result maps to Anthropic user message with tool_result block
                let content_str = msg.content.as_ref()
                    .and_then(|c| if c.is_string() { c.as_str().map(|s| s.to_string()) } else { Some(c.to_string()) })
                    .unwrap_or_default();

                let tool_call_id = msg.tool_call_id.clone().unwrap_or_else(|| "tool_call".to_string());
                messages.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_call_id,
                        "content": content_str
                    }]
                }));
            } else {
                let mut blocks = Vec::new();
                if let Some(c) = &msg.content {
                    if let Some(s) = c.as_str() {
                        blocks.push(json!({
                            "type": "text",
                            "text": s
                        }));
                    } else if let Some(arr) = c.as_array() {
                        blocks.extend(arr.clone());
                    }
                }

                if let Some(tool_calls) = &msg.tool_calls {
                    for tc in tool_calls {
                        let id = tc.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let function = tc.get("function").cloned().unwrap_or(Value::Null);
                        let name = function.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let args_str = function.get("arguments").and_then(|a| a.as_str()).unwrap_or("{}");
                        let input: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                        blocks.push(json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input
                        }));
                    }
                }

                let role = if msg.role == "assistant" { "assistant" } else { "user" };
                messages.push(json!({
                    "role": role,
                    "content": blocks
                }));
            }
        }

        let max_tokens = req.max_tokens.unwrap_or(4096);
        let mut payload = json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "stream": req.stream,
        });

        if !system.is_empty() {
            payload["system"] = json!(system);
        }
        if let Some(temp) = req.temperature {
            payload["temperature"] = json!(temp);
        }

        payload
    }

    pub fn anthropic_to_openai_response(val: &Value) -> Value {
        let content_text = val.get("content")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|block| {
                        if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                            block.get("text").and_then(|t| t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        let id = val.get("id").and_then(|i| i.as_str()).unwrap_or("chatcmpl-claude");
        let model = val.get("model").and_then(|m| m.as_str()).unwrap_or("claude");

        json!({
            "id": id,
            "object": "chat.completion",
            "created": chrono::Utc::now().timestamp(),
            "model": model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content_text
                },
                "finish_reason": "stop"
            }]
        })
    }
}
