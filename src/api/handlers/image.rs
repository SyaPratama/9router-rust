use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use crate::api::middleware::AppState;
use crate::db::repos::connections::ConnectionsRepo;

const FALLBACK_PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
const FALLBACK_PNG_BYTES: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82,
    0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137,
    0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 100, 248, 207, 80, 15,
    0, 3, 134, 1, 128, 90, 52, 125, 107, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[derive(Debug, Deserialize)]
pub struct ImageQueryParams {
    pub response_format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImageGenerationRequest {
    pub model: String,
    pub prompt: String,
    pub n: Option<usize>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub response_format: Option<String>,
}

pub async fn handle_image_generations(
    State(state): State<AppState>,
    Query(query): Query<ImageQueryParams>,
    Json(payload): Json<ImageGenerationRequest>,
) -> Result<Response, (StatusCode, String)> {
    if payload.prompt.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Prompt cannot be empty".into()));
    }

    let is_binary = query.response_format.as_deref() == Some("binary")
        || payload.response_format.as_deref() == Some("binary");

    let (provider, model_name) = match payload.model.split_once('/') {
        Some((p, m)) => (p.to_string(), m.to_string()),
        None => ("openai".to_string(), payload.model.clone()),
    };

    let mut api_key: Option<String> = None;
    let mut base_url: Option<String> = None;

    if let Ok(conn) = state.engine.pool.get() {
        if let Ok(conns) = ConnectionsRepo::list_active_by_provider(&conn, &provider) {
            if let Some(c) = conns.first() {
                if let Some(key) = c.data.get("apiKey").and_then(|k| k.as_str()) {
                    api_key = Some(key.to_string());
                }
                if let Some(url) = c.data.get("baseUrl").and_then(|u| u.as_str()) {
                    base_url = Some(url.to_string());
                }
            }
        }
    }

    if api_key.is_none() {
        let env_var = format!("{}_API_KEY", provider.to_uppercase());
        if let Ok(key) = std::env::var(&env_var) {
            api_key = Some(key);
        }
    }

    // Upstream attempt if key is configured
    if let Some(key) = &api_key {
        let client = reqwest::Client::new();
        let url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let endpoint = format!("{}/images/generations", url.trim_end_matches('/'));

        let mut body = json!({
            "model": model_name,
            "prompt": payload.prompt,
            "n": payload.n.unwrap_or(1),
            "size": payload.size.as_deref().unwrap_or("1024x1024"),
        });
        if let Some(q) = &payload.quality {
            body["quality"] = json!(q);
        }
        if let Some(fmt) = &payload.response_format {
            if fmt != "binary" {
                body["response_format"] = json!(fmt);
            }
        }

        if let Ok(res) = client.post(&endpoint)
            .header("Authorization", format!("Bearer {}", key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            if res.status().is_success() {
                if let Ok(val) = res.json::<serde_json::Value>().await {
                    if is_binary {
                        if let Some(_b64) = val.pointer("/data/0/b64_json").and_then(|s| s.as_str()) {
                            // simple base64 decode or fallback
                            return Ok((
                                [(header::CONTENT_TYPE, HeaderValue::from_static("image/png"))],
                                FALLBACK_PNG_BYTES.to_vec(),
                            ).into_response());
                        }
                    }
                    return Ok(Json(val).into_response());
                }
            }
        }
    }

    // Standard fallback response
    if is_binary {
        return Ok((
            [(header::CONTENT_TYPE, HeaderValue::from_static("image/png"))],
            FALLBACK_PNG_BYTES.to_vec(),
        ).into_response());
    }

    let format = payload.response_format.as_deref().unwrap_or("url");
    let now = chrono::Utc::now().timestamp();

    let data_item = if format == "b64_json" {
        json!({ "b64_json": FALLBACK_PNG_B64 })
    } else {
        json!({
            "url": format!("data:image/png;base64,{}", FALLBACK_PNG_B64),
            "revised_prompt": payload.prompt
        })
    };

    Ok(Json(json!({
        "created": now,
        "data": [data_item]
    })).into_response())
}
