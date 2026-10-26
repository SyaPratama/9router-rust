use super::types::ChatRequest;
use serde_json::Value;

pub struct OpenAiTranslator;

impl OpenAiTranslator {
    pub fn to_openai_payload(req: &ChatRequest) -> Value {
        serde_json::to_value(req).unwrap_or(Value::Null)
    }

    pub fn parse_openai_chunk(chunk_line: &str) -> Option<Value> {
        let trimmed = chunk_line.trim();
        if !trimmed.starts_with("data: ") {
            return None;
        }
        let data = &trimmed[6..].trim();
        if *data == "[DONE]" {
            return None;
        }
        serde_json::from_str(data).ok()
    }
}
