use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::api::middleware::AppState;
use crate::db::repos::connections::ConnectionsRepo;

#[derive(Debug, Deserialize)]
pub struct EmbeddingsRequest {
    pub model: String,
    pub input: serde_json::Value, // String or Vec<String>
    pub encoding_format: Option<String>,
    pub dimensions: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingItem {
    pub object: String,
    pub index: usize,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingsUsage {
    pub prompt_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingsResponse {
    pub object: String,
    pub model: String,
    pub data: Vec<EmbeddingItem>,
    pub usage: EmbeddingsUsage,
}

// Generate deterministic pseudo-embedding vector for offline / testing fallback
fn generate_fallback_embedding(text: &str, dims: usize) -> Vec<f32> {
    use sha2::{Digest, Sha256};
    let mut vec = Vec::with_capacity(dims);
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let hash = hasher.finalize();

    for i in 0..dims {
        let b = hash[i % hash.len()] as f32;
        let v = ((b / 255.0) - 0.5) * 2.0;
        vec.push(v);
    }
    // L2 normalize
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut vec {
            *x /= norm;
        }
    }
    vec
}

pub async fn handle_embeddings(
    State(state): State<AppState>,
    Json(payload): Json<EmbeddingsRequest>,
) -> Result<Response, (StatusCode, String)> {
    let inputs: Vec<String> = match &payload.input {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => return Err((StatusCode::BAD_REQUEST, "Invalid input format: string or string array required".into())),
    };

    if inputs.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Input array cannot be empty".into()));
    }

    let dims = payload.dimensions.unwrap_or(1536);
    let (provider, model_name) = match payload.model.split_once('/') {
        Some((p, m)) => (p.to_string(), m.to_string()),
        None => ("openai".to_string(), payload.model.clone()),
    };

    // Check upstream credentials from DB or Env
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

    // Upstream call if credentials exist
    if let Some(key) = &api_key {
        let client = reqwest::Client::new();
        if provider == "gemini" || provider == "google_ai_studio" {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:batchEmbedContents?key={}",
                model_name, key
            );
            let requests: Vec<serde_json::Value> = inputs
                .iter()
                .map(|t| {
                    json!({
                        "model": format!("models/{}", model_name),
                        "content": { "parts": [{ "text": t }] }
                    })
                })
                .collect();

            if let Ok(res) = client.post(&url).json(&json!({ "requests": requests })).send().await {
                if res.status().is_success() {
                    if let Ok(gemini_json) = res.json::<serde_json::Value>().await {
                        if let Some(embeddings_arr) = gemini_json.get("embeddings").and_then(|e| e.as_array()) {
                            let data: Vec<EmbeddingItem> = embeddings_arr
                                .iter()
                                .enumerate()
                                .map(|(i, item)| {
                                    let values: Vec<f32> = item
                                        .get("values")
                                        .and_then(|v| v.as_array())
                                        .map(|arr| arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())
                                        .unwrap_or_default();
                                    EmbeddingItem {
                                        object: "embedding".to_string(),
                                        index: i,
                                        embedding: values,
                                    }
                                })
                                .collect();

                            let total_tokens = inputs.iter().map(|s| s.len() / 4).sum::<usize>().max(1);
                            return Ok(Json(EmbeddingsResponse {
                                object: "list".to_string(),
                                model: payload.model.clone(),
                                data,
                                usage: EmbeddingsUsage {
                                    prompt_tokens: total_tokens,
                                    total_tokens,
                                },
                            }).into_response());
                        }
                    }
                }
            }
        } else {
            // OpenAI / compatible
            let url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let endpoint = format!("{}/embeddings", url.trim_end_matches('/'));
            let mut body = json!({
                "model": model_name,
                "input": inputs,
            });
            if let Some(d) = payload.dimensions {
                body["dimensions"] = json!(d);
            }
            if let Some(f) = &payload.encoding_format {
                body["encoding_format"] = json!(f);
            }

            if let Ok(res) = client.post(&endpoint)
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                if res.status().is_success() {
                    if let Ok(resp_json) = res.json::<serde_json::Value>().await {
                        return Ok(Json(resp_json).into_response());
                    }
                }
            }
        }
    }

    // Deterministic fallback response when offline / keys not supplied
    let mut data = Vec::with_capacity(inputs.len());
    let mut total_tokens = 0;
    for (i, text) in inputs.iter().enumerate() {
        let embedding = generate_fallback_embedding(text, dims);
        let tokens = (text.len() / 4).max(1);
        total_tokens += tokens;
        data.push(EmbeddingItem {
            object: "embedding".to_string(),
            index: i,
            embedding,
        });
    }

    Ok(Json(json!({
        "object": "list",
        "model": payload.model,
        "data": data,
        "usage": {
            "prompt_tokens": total_tokens,
            "total_tokens": total_tokens
        }
    })).into_response())
}
