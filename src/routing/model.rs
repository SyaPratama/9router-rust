pub struct ParsedModel {
    pub provider: String,
    pub model: String,
}

pub fn parse_model(raw: &str) -> ParsedModel {
    if let Some((provider, model)) = raw.split_once('/') {
        ParsedModel {
            provider: provider.to_lowercase(),
            model: model.to_string(),
        }
    } else {
        // Auto-detect provider from prefix or default to openai
        let lower = raw.to_lowercase();
        let provider = if lower.starts_with("claude") {
            "anthropic"
        } else if lower.starts_with("gemini") {
            "gemini"
        } else if lower.starts_with("deepseek") {
            "deepseek"
        } else if lower.starts_with("qwen") {
            "openrouter"
        } else {
            "openai"
        };

        ParsedModel {
            provider: provider.to_string(),
            model: raw.to_string(),
        }
    }
}
