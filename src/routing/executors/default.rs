use reqwest::Client;
use serde_json::Value;

pub struct DefaultExecutor;

impl DefaultExecutor {
    pub async fn execute_chat(
        client: &Client,
        base_url: &str,
        api_key: &str,
        payload: &Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let endpoint = if base_url.ends_with('/') {
            format!("{}chat/completions", base_url)
        } else {
            format!("{}/chat/completions", base_url)
        };

        client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await
    }
}
