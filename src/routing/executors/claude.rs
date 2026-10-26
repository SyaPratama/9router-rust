use reqwest::Client;
use serde_json::Value;

pub struct ClaudeExecutor;

impl ClaudeExecutor {
    pub async fn execute_messages(
        client: &Client,
        base_url: &str,
        api_key: &str,
        payload: &Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let endpoint = if base_url.ends_with('/') {
            format!("{}v1/messages", base_url)
        } else {
            format!("{}/v1/messages", base_url)
        };

        client
            .post(&endpoint)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await
    }
}
