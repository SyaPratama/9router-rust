use reqwest::Client;
use serde_json::Value;

pub struct GeminiExecutor;

impl GeminiExecutor {
    pub async fn execute_generate(
        client: &Client,
        base_url: &str,
        api_key: &str,
        model: &str,
        payload: &Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let endpoint = if base_url.ends_with('/') {
            format!("{}models/{}:generateContent?key={}", base_url, model, api_key)
        } else {
            format!("{}/models/{}:generateContent?key={}", base_url, model, api_key)
        };

        client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await
    }
}
