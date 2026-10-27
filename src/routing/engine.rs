use crate::db::repos::settings::SettingsRepo;
use crate::db::DbPool;
use crate::headroom::HeadroomPipeline;
use crate::routing::account::AccountManager;
use crate::routing::combo::resolve_model_chain;
use crate::routing::executors::{ClaudeExecutor, DefaultExecutor, GeminiExecutor};
use crate::routing::model::parse_model;
use crate::routing::token_saver::TokenSaverPipeline;
use crate::translator::{ChatRequest, ClaudeTranslator, GeminiTranslator, OpenAiTranslator};
use reqwest::Client;
use std::sync::Arc;

#[derive(Clone)]
pub struct RoutingEngine {
    pub pool: DbPool,
    pub http_client: Client,
    pub account_mgr: AccountManager,
    pub headroom: Arc<HeadroomPipeline>,
}
pub struct DispatchResult {
    pub response: reqwest::Response,
    pub provider: String,
    pub model: String,
    pub connection_id: Option<String>,
}


impl RoutingEngine {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            http_client: Client::builder().build().unwrap_or_default(),
            account_mgr: AccountManager::new(),
            headroom: Arc::new(HeadroomPipeline::new()),
        }
    }

    pub async fn dispatch_chat(&self, mut req: ChatRequest) -> Result<DispatchResult, (axum::http::StatusCode, String)> {
        let conn = self.pool.get().map_err(|e| {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB connection error: {}", e))
        })?;

        let settings = SettingsRepo::get(&conn).unwrap_or_default();
        let candidate_models = resolve_model_chain(&conn, &req.model);

        // Apply TokenSaverPipeline (Headroom + Caveman + Ponytail)
        TokenSaverPipeline::process(
            &mut req.messages,
            &self.headroom,
            settings.headroom_enabled,
            settings.caveman_level.as_deref(),
            settings.ponytail_level.as_deref(),
        );

        let mut last_error = String::from("No upstream accounts available");

        // Combo model loop
        for model_id in candidate_models {
            let parsed = parse_model(&model_id);
            let accounts = self.account_mgr.get_available_connections(&conn, &parsed.provider);

            if accounts.is_empty() {
                // If no connection in DB, check if provider has env key (e.g. OPENAI_API_KEY)
                let env_key_var = format!("{}_API_KEY", parsed.provider.to_uppercase());
                if let Ok(env_key) = std::env::var(&env_key_var) {
                    let base_url = std::env::var(format!("{}_BASE_URL", parsed.provider.to_uppercase()))
                        .unwrap_or_else(|_| match parsed.provider.as_str() {
                            "anthropic" => "https://api.anthropic.com".to_string(),
                            "gemini" => "https://generativelanguage.googleapis.com/v1beta/".to_string(),
                            _ => "https://api.openai.com/v1".to_string(),
                        });

                    let mut sub_req = req.clone();
                    sub_req.model = parsed.model.clone();

                    let res = self.send_to_upstream(&parsed.provider, &base_url, &env_key, &sub_req).await;
                    match res {
                        Ok(resp) if resp.status().is_success() => return Ok(DispatchResult {
                            response: resp, provider: parsed.provider, model: parsed.model, connection_id: None,
                        }),
                        Ok(resp) => {
                            last_error = format!("Upstream HTTP error: {}", resp.status());
                        }
                        Err(e) => {
                            last_error = format!("Upstream network error: {}", e);
                        }
                    }
                }
                continue;
            }

            // Account fallback loop
            for account in accounts {
                let api_key = account.data.get("apiKey")
                    .and_then(|k| k.as_str())
                    .unwrap_or("")
                    .to_string();

                let base_url = account.data.get("baseUrl")
                    .and_then(|b| b.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| match parsed.provider.as_str() {
                        "anthropic" => "https://api.anthropic.com".to_string(),
                        "gemini" => "https://generativelanguage.googleapis.com/v1beta/".to_string(),
                        _ => "https://api.openai.com/v1".to_string(),
                    });

                let mut sub_req = req.clone();
                sub_req.model = parsed.model.clone();

                let res = self.send_to_upstream(&parsed.provider, &base_url, &api_key, &sub_req).await;
                match res {
                    Ok(resp) => {
                        let status = resp.status();
                        if status.is_success() {
                            self.account_mgr.clear_strikes(&account.id);
                            return Ok(DispatchResult {
                                response: resp, provider: parsed.provider, model: parsed.model, connection_id: Some(account.id.clone()),
                            });
                        } else if status.as_u16() == 429 || status.is_server_error() {
                            // Rate limit or temporary error: strike and fallback to next account
                            self.account_mgr.record_strike(&account.id);
                            last_error = format!("Account {} failed with status {}", account.id, status);
                            continue;
                        } else {
                            // Client error (e.g. 400 Bad Request): return immediately
                            return Ok(DispatchResult {
                                response: resp, provider: parsed.provider, model: parsed.model, connection_id: Some(account.id.clone()),
                            });
                        }
                    }
                    Err(e) => {
                        self.account_mgr.record_strike(&account.id);
                        last_error = format!("Account {} network error: {}", account.id, e);
                        continue;
                    }
                }
            }
        }

        Err((axum::http::StatusCode::BAD_GATEWAY, last_error))
    }

    async fn send_to_upstream(
        &self,
        provider: &str,
        base_url: &str,
        api_key: &str,
        req: &ChatRequest,
    ) -> Result<reqwest::Response, reqwest::Error> {
        match provider {
            "anthropic" => {
                let payload = ClaudeTranslator::to_anthropic_payload(req);
                ClaudeExecutor::execute_messages(&self.http_client, base_url, api_key, &payload).await
            }
            "gemini" => {
                let payload = GeminiTranslator::to_gemini_payload(req);
                GeminiExecutor::execute_generate(&self.http_client, base_url, api_key, &req.model, &payload).await
            }
            _ => {
                let payload = OpenAiTranslator::to_openai_payload(req);
                DefaultExecutor::execute_chat(&self.http_client, base_url, api_key, &payload).await
            }
        }
    }
}
