use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use crate::api::middleware::AppState;
use crate::db::repos::combos::CombosRepo;
use crate::db::repos::connections::ConnectionsRepo;

#[derive(Debug, Deserialize)]
pub struct ModelInfoQuery {
    pub id: Option<String>,
}

fn base_catalog() -> Vec<serde_json::Value> {
    vec![
        // Chat
        json!({"id": "openai/gpt-4o", "object": "model", "kind": "chat", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "openai/gpt-4o-mini", "object": "model", "kind": "chat", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "openai/gpt-5", "object": "model", "kind": "chat", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "anthropic/claude-3-5-sonnet", "object": "model", "kind": "chat", "owned_by": "anthropic", "created": 1735000000}),
        json!({"id": "anthropic/claude-3-5-haiku", "object": "model", "kind": "chat", "owned_by": "anthropic", "created": 1735000000}),
        json!({"id": "cc/claude-opus-4-7", "object": "model", "kind": "chat", "owned_by": "anthropic", "created": 1735000000}),
        json!({"id": "deepseek/deepseek-chat", "object": "model", "kind": "chat", "owned_by": "deepseek", "created": 1735000000}),
        json!({"id": "deepseek/deepseek-reasoner", "object": "model", "kind": "chat", "owned_by": "deepseek", "created": 1735000000}),
        json!({"id": "gemini/gemini-1.5-pro", "object": "model", "kind": "chat", "owned_by": "gemini", "created": 1735000000}),
        json!({"id": "gemini/gemini-1.5-flash", "object": "model", "kind": "chat", "owned_by": "gemini", "created": 1735000000}),
        json!({"id": "gemini/gemini-2.5-flash", "object": "model", "kind": "chat", "owned_by": "gemini", "created": 1735000000}),
        json!({"id": "gemini/gemini-2.5-pro", "object": "model", "kind": "chat", "owned_by": "gemini", "created": 1735000000}),

        // Image
        json!({"id": "openai/dall-e-3", "object": "model", "kind": "image", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "openai/dall-e-2", "object": "model", "kind": "image", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "gemini/imagen-3", "object": "model", "kind": "image", "owned_by": "gemini", "created": 1735000000}),
        json!({"id": "gemini/gemini-3-pro-image-preview", "object": "model", "kind": "image", "owned_by": "gemini", "created": 1735000000}),
        json!({"id": "black-forest-labs/flux-1.1-pro", "object": "model", "kind": "image", "owned_by": "black-forest-labs", "created": 1735000000}),
        json!({"id": "black-forest-labs/flux-schnell", "object": "model", "kind": "image", "owned_by": "black-forest-labs", "created": 1735000000}),
        json!({"id": "stability-ai/stable-diffusion-3-medium", "object": "model", "kind": "image", "owned_by": "stability-ai", "created": 1735000000}),
        json!({"id": "minimax/image-01", "object": "model", "kind": "image", "owned_by": "minimax", "created": 1735000000}),

        // TTS
        json!({"id": "openai/tts-1", "object": "model", "kind": "tts", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "openai/tts-1-hd", "object": "model", "kind": "tts", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "el/eleven_multilingual_v2", "object": "model", "kind": "tts", "owned_by": "elevenlabs", "created": 1735000000}),
        json!({"id": "el/eleven_flash_v2_5", "object": "model", "kind": "tts", "owned_by": "elevenlabs", "created": 1735000000}),
        json!({"id": "edge-tts/vi-VN-HoaiMyNeural", "object": "model", "kind": "tts", "owned_by": "edge-tts", "created": 1735000000}),
        json!({"id": "edge-tts/en-US-GuyNeural", "object": "model", "kind": "tts", "owned_by": "edge-tts", "created": 1735000000}),
        json!({"id": "edge-tts/en-US-JennyNeural", "object": "model", "kind": "tts", "owned_by": "edge-tts", "created": 1735000000}),
        json!({"id": "google-tts/en", "object": "model", "kind": "tts", "owned_by": "google-tts", "created": 1735000000}),
        json!({"id": "google-tts/vi", "object": "model", "kind": "tts", "owned_by": "google-tts", "created": 1735000000}),
        json!({"id": "deepgram/aura-asteria-en", "object": "model", "kind": "tts", "owned_by": "deepgram", "created": 1735000000}),

        // STT
        json!({"id": "openai/whisper-1", "object": "model", "kind": "stt", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "openai/gpt-4o-transcribe", "object": "model", "kind": "stt", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "groq/whisper-large-v3", "object": "model", "kind": "stt", "owned_by": "groq", "created": 1735000000}),
        json!({"id": "groq/whisper-large-v3-turbo", "object": "model", "kind": "stt", "owned_by": "groq", "created": 1735000000}),
        json!({"id": "deepgram/nova-3", "object": "model", "kind": "stt", "owned_by": "deepgram", "created": 1735000000}),
        json!({"id": "deepgram/nova-2", "object": "model", "kind": "stt", "owned_by": "deepgram", "created": 1735000000}),
        json!({"id": "elevenlabs/scribe_v1", "object": "model", "kind": "stt", "owned_by": "elevenlabs", "created": 1735000000}),

        // Embedding
        json!({"id": "openai/text-embedding-3-small", "object": "model", "kind": "embedding", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "openai/text-embedding-3-large", "object": "model", "kind": "embedding", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "openai/text-embedding-ada-002", "object": "model", "kind": "embedding", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "gemini/text-embedding-004", "object": "model", "kind": "embedding", "owned_by": "gemini", "created": 1735000000}),
        json!({"id": "mistral/mistral-embed", "object": "model", "kind": "embedding", "owned_by": "mistral", "created": 1735000000}),
        json!({"id": "voyage-ai/voyage-3", "object": "model", "kind": "embedding", "owned_by": "voyage-ai", "created": 1735000000}),
        json!({"id": "jina-ai/jina-embeddings-v3", "object": "model", "kind": "embedding", "owned_by": "jina-ai", "created": 1735000000}),

        // Video
        json!({"id": "xai/grok-imagine-video", "object": "model", "kind": "video", "owned_by": "xai", "created": 1735000000}),
        json!({"id": "runwayml/gen3-alpha", "object": "model", "kind": "video", "owned_by": "runwayml", "created": 1735000000}),

        // Web Search
        json!({"id": "tavily/search", "object": "model", "kind": "webSearch", "owned_by": "tavily", "created": 1735000000}),
        json!({"id": "brave/search", "object": "model", "kind": "webSearch", "owned_by": "brave", "created": 1735000000}),
        json!({"id": "serper/search", "object": "model", "kind": "webSearch", "owned_by": "serper", "created": 1735000000}),
        json!({"id": "searxng/search", "object": "model", "kind": "webSearch", "owned_by": "searxng", "created": 1735000000}),
        json!({"id": "exa/search", "object": "model", "kind": "webSearch", "owned_by": "exa", "created": 1735000000}),
        json!({"id": "xquik/search", "object": "model", "kind": "webSearch", "owned_by": "xquik", "created": 1735000000}),

        // Web Fetch
        json!({"id": "jina-reader/fetch", "object": "model", "kind": "webFetch", "owned_by": "jina-reader", "created": 1735000000}),
        json!({"id": "firecrawl/fetch", "object": "model", "kind": "webFetch", "owned_by": "firecrawl", "created": 1735000000}),
        json!({"id": "tavily/fetch", "object": "model", "kind": "webFetch", "owned_by": "tavily", "created": 1735000000}),
        json!({"id": "exa/fetch", "object": "model", "kind": "webFetch", "owned_by": "exa", "created": 1735000000}),
        json!({"id": "ollama/fetch", "object": "model", "kind": "webFetch", "owned_by": "ollama", "created": 1735000000}),

        // Vision / image-to-text
        json!({"id": "openai/gpt-4o-vision", "object": "model", "kind": "image-to-text", "owned_by": "openai", "created": 1735000000}),
        json!({"id": "gemini/gemini-1.5-pro-vision", "object": "model", "kind": "image-to-text", "owned_by": "gemini", "created": 1735000000}),
    ]
}

pub fn get_all_models(state: &AppState) -> Vec<serde_json::Value> {
    let mut model_list = base_catalog();

    if let Ok(conn) = state.engine.pool.get() {
        if let Ok(conns) = ConnectionsRepo::list_all(&conn) {
            for c in conns {
                if c.is_active {
                    model_list.push(json!({
                        "id": format!("{}/auto", c.provider),
                        "object": "model",
                        "kind": "chat",
                        "owned_by": c.provider,
                        "created": 1735000000
                    }));
                }
            }
        }

        if let Ok(combos) = CombosRepo::list_all(&conn) {
            for combo in combos {
                let kind = combo.kind.unwrap_or_else(|| "chat".to_string());
                model_list.push(json!({
                    "id": combo.name,
                    "object": "model",
                    "kind": kind,
                    "owned_by": "combo",
                    "created": 1735000000
                }));
            }
        }
    }

    model_list
}

pub async fn list_models(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let model_list = get_all_models(&state);
    Ok(Json(json!({
        "object": "list",
        "data": model_list
    })))
}

pub async fn get_model_or_kind(
    State(state): State<AppState>,
    Path(kind_or_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let all = get_all_models(&state);

    let is_kind_query = matches!(
        kind_or_id.as_str(),
        "image"
            | "tts"
            | "stt"
            | "embedding"
            | "video"
            | "web"
            | "webSearch"
            | "webFetch"
            | "image-to-text"
            | "chat"
    );

    if is_kind_query {
        let filtered: Vec<serde_json::Value> = all
            .into_iter()
            .filter(|m| {
                let m_kind = m.get("kind").and_then(|k| k.as_str()).unwrap_or("chat");
                if kind_or_id == "web" {
                    m_kind == "webSearch" || m_kind == "webFetch" || m_kind == "web"
                } else {
                    m_kind == kind_or_id
                }
            })
            .collect();

        return Ok(Json(json!({
            "object": "list",
            "data": filtered
        })));
    }

    // Single model lookup
    if let Some(found) = all.iter().find(|m| m.get("id").and_then(|id| id.as_str()) == Some(&kind_or_id)) {
        return Ok(Json(found.clone()));
    }

    Ok(Json(json!({
        "id": kind_or_id,
        "object": "model",
        "created": 1735000000,
        "owned_by": "9router"
    })))
}

pub async fn get_model_info(
    Query(query): Query<ModelInfoQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let model_id = query.id.unwrap_or_else(|| "openai/gpt-4o".to_string());

    let (kind, context_window, max_output, dimensions, params) = if model_id.contains("embedding") {
        ("embedding", 8192, 0, 1536, json!({"dimensions": [512, 1536]}))
    } else if model_id.contains("dall-e") || model_id.contains("imagen") || model_id.contains("flux") {
        ("image", 1000, 0, 0, json!({"sizes": ["1024x1024", "1792x1024"], "qualities": ["standard", "hd"]}))
    } else if model_id.contains("tts") || model_id.contains("eleven") || model_id.contains("edge-tts") {
        ("tts", 4096, 0, 0, json!({"formats": ["mp3", "opus", "aac", "flac"]}))
    } else if model_id.contains("whisper") || model_id.contains("stt") || model_id.contains("scribe") {
        ("stt", 25000000, 0, 0, json!({"formats": ["json", "verbose_json", "text", "srt", "vtt"]}))
    } else if model_id.contains("video") || model_id.contains("imagine") {
        ("video", 2000, 0, 0, json!({"aspect_ratios": ["16:9", "9:16", "1:1"], "durations": [5, 10, 15]}))
    } else if model_id.contains("search") || model_id.contains("tavily") || model_id.contains("brave") {
        ("webSearch", 1000, 0, 0, json!({"search_types": ["web", "news"], "max_results": 20}))
    } else if model_id.contains("fetch") || model_id.contains("jina") || model_id.contains("firecrawl") {
        ("webFetch", 50000, 0, 0, json!({"formats": ["markdown", "text", "html"]}))
    } else {
        ("chat", 128000, 16384, 0, json!({"temperature": [0.0, 2.0], "top_p": [0.0, 1.0]}))
    };

    Ok(Json(json!({
        "id": model_id,
        "object": "model_info",
        "kind": kind,
        "context_window": context_window,
        "max_output_tokens": max_output,
        "dimensions": if dimensions > 0 { Some(dimensions) } else { None },
        "parameters": params,
        "pricing": {
            "prompt": 0.000005,
            "completion": 0.000015
        },
        "supported_formats": ["json", "stream"]
    })))
}
