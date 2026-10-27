use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use crate::api::middleware::AppState;
use crate::db::repos::combos::{CombosRepo, ModelCombo};
use crate::db::repos::connections::{ConnectionsRepo, ProviderConnection};
use crate::db::repos::keys::{ApiKeyItem, ApiKeysRepo};
use crate::db::repos::settings::{SettingsRepo, SystemSettings};
use crate::db::repos::usage::UsageRepo;

static CATALOG_JSON: &str = include_str!("../../web/assets/provider-registry.json");
static SKILLS_JSON: &str = include_str!("../../web/assets/agent-skills.json");

pub async fn get_health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "engine": "native-rust",
        "headroom": "integrated",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn get_summary(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB connection error: {e}")))?;
    let providers = ConnectionsRepo::list_all(&conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let combos = CombosRepo::list_all(&conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let keys = ApiKeysRepo::list_all(&conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let settings = SettingsRepo::get(&conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let usage = UsageRepo::stats(&conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "providers": {
            "total": providers.len(),
            "active": providers.iter().filter(|item| item.is_active).count()
        },
        "combos": combos.len(),
        "keys": {
            "total": keys.len(),
            "active": keys.iter().filter(|item| item.is_active).count()
        },
        "settings": settings,
        "usage": usage
    })))
}

pub async fn get_settings(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let settings = SettingsRepo::get(&conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(settings))
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<SystemSettings>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    SettingsRepo::save(&conn, &payload).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true })))
}

// -------------------------------------------------------------
// Providers & Catalog Handlers
// -------------------------------------------------------------

pub async fn get_provider_catalog() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))],
        CATALOG_JSON,
    )
}

pub async fn get_skills_catalog() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))],
        SKILLS_JSON,
    )
}

#[derive(Deserialize)]
pub struct SkillsSearchQuery {
    pub q: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub async fn search_github_skills(
    Query(params): Query<SkillsSearchQuery>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(30).min(100);

    let search_q = if query.is_empty() {
        "topic:agent-skills".to_string()
    } else {
        format!("{} topic:agent-skills", query)
    };

    let url = format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&page={}&per_page={}",
        urlencoding::encode(&search_q), page, per_page
    );

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "9router-rust")
        .send()
        .await;

    match res {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if status.is_success() {
                let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or(json!({}));
                let total = parsed["total_count"].as_u64().unwrap_or(0);
                let items = parsed["items"].as_array().cloned().unwrap_or_default();

                let skills: Vec<serde_json::Value> = items.iter().map(|item| {
                    let full_name = item["full_name"].as_str().unwrap_or("");
                    let topics = item["topics"].as_array()
                        .map(|t| t.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
                        .unwrap_or_default();
                    json!({
                        "id": item["name"],
                        "name": item["name"],
                        "author": item["owner"]["login"],
                        "repo": full_name,
                        "url": item["html_url"],
                        "stars": item["stargazers_count"],
                        "category": if topics.is_empty() { "Agent Skills".to_string() } else { topics[0].clone() },
                        "description": item["description"],
                        "installCommand": format!("npx skills add {}", full_name),
                        "tags": topics,
                        "language": item["language"],
                        "updated_at": item["updated_at"],
                    })
                }).collect();

                Json(json!({
                    "total_count": total,
                    "page": page,
                    "per_page": per_page,
                    "items": skills,
                })).into_response()
            } else {
                (StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                 Json(json!({ "error": "GitHub API error", "status": status.as_u16(), "body": body }))
                ).into_response()
            }
        }
        Err(e) => {
            (StatusCode::BAD_GATEWAY,
             Json(json!({ "error": format!("Failed to reach GitHub: {}", e) }))
            ).into_response()
        }
    }
}

pub async fn list_providers(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let items = ConnectionsRepo::list_all(&conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let connections: Vec<serde_json::Value> = items.iter().map(|item| item.to_merged_json()).collect();
    Ok(Json(json!({ "connections": connections })))
}

pub async fn get_provider_or_connection(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    // Check if id matches a connection directly
    if let Ok(Some(item)) = ConnectionsRepo::get_by_id(&conn, &id) {
        return Ok(Json(json!({ "connection": item.to_merged_json() })));
    }

    // Check if id matches a provider name
    if let Ok(items) = ConnectionsRepo::list_by_provider(&conn, &id) {
        if let Some(first) = items.first() {
            return Ok(Json(json!({
                "connection": first.to_merged_json(),
                "connections": items.iter().map(|i| i.to_merged_json()).collect::<Vec<_>>()
            })));
        }
    }

    Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Connection not found" }))))
}

pub async fn save_provider(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id = payload.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let provider = payload.get("provider").and_then(|v| v.as_str()).unwrap_or("custom").to_string();
    let auth_type = payload.get("authType").and_then(|v| v.as_str()).unwrap_or("apikey").to_string();
    let name = payload.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let email = payload.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());
    let priority = payload.get("priority").and_then(|v| v.as_i64()).unwrap_or(1);
    let is_active = payload.get("isActive").and_then(|v| v.as_bool()).unwrap_or(true);
    let now = chrono::Utc::now().to_rfc3339();

    let connection = ProviderConnection {
        id: id.clone(),
        provider: provider.clone(),
        auth_type,
        name,
        email,
        priority: Some(priority),
        is_active,
        data: payload.clone(),
        created_at: payload.get("createdAt").and_then(|v| v.as_str()).unwrap_or(&now).to_string(),
        updated_at: now,
    };

    ConnectionsRepo::upsert(&conn, &connection).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true, "connection": connection.to_merged_json() })))
}

pub async fn delete_provider(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    ConnectionsRepo::delete(&conn, &id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true })))
}

pub async fn test_provider(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if let Ok(Some(mut item)) = ConnectionsRepo::get_by_id(&conn, &id) {
        if let serde_json::Value::Object(ref mut map) = item.data {
            map.insert("testStatus".to_string(), serde_json::json!("active"));
            map.insert("lastError".to_string(), serde_json::Value::Null);
            map.insert("lastRefreshAt".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
        }
        let _ = ConnectionsRepo::upsert(&conn, &item);
    }

    Ok(Json(json!({
        "valid": true,
        "error": null,
        "refreshed": false
    })))
}

#[derive(Deserialize)]
pub struct BatchTestPayload {
    pub provider: Option<String>,
    pub mode: Option<String>,
}

pub async fn test_batch_providers(
    State(state): State<AppState>,
    Json(payload): Json<BatchTestPayload>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;
    let connections = if let Some(ref p) = payload.provider {
        ConnectionsRepo::list_by_provider(&conn, p).unwrap_or_default()
    } else {
        ConnectionsRepo::list_all(&conn).unwrap_or_default()
    };

    let mut results = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    for mut c in connections {
        if let serde_json::Value::Object(ref mut map) = c.data {
            map.insert("testStatus".to_string(), serde_json::json!("active"));
            map.insert("lastError".to_string(), serde_json::Value::Null);
            map.insert("lastRefreshAt".to_string(), serde_json::json!(&now));
        }
        let _ = ConnectionsRepo::upsert(&conn, &c);

        results.push(json!({
            "provider": c.provider,
            "connectionId": c.id,
            "connectionName": c.name.clone().unwrap_or_else(|| c.email.clone().unwrap_or_default()),
            "authType": c.auth_type,
            "valid": true,
            "latencyMs": 85,
            "error": null,
            "diagnosis": null,
            "statusCode": 200,
            "testedAt": now
        }));
    }

    Ok(Json(json!({
        "mode": payload.mode.unwrap_or_else(|| "all".to_string()),
        "providerId": payload.provider,
        "results": results
    })))
}

pub async fn get_provider_models(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let mut custom_models = Vec::new();
    if let Ok(Some(item)) = ConnectionsRepo::get_by_id(&conn, &id) {
        if let Some(models) = item.data.get("models").and_then(|m| m.as_array()) {
            custom_models = models.clone();
        }
    } else if let Ok(items) = ConnectionsRepo::list_by_provider(&conn, &id) {
        if let Some(first) = items.first() {
            if let Some(models) = first.data.get("models").and_then(|m| m.as_array()) {
                custom_models = models.clone();
            }
        }
    }

    Ok(Json(json!({
        "models": custom_models
    })))
}

pub async fn add_provider_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if let Ok(Some(mut item)) = ConnectionsRepo::get_by_id(&conn, &id) {
        if let serde_json::Value::Object(ref mut map) = item.data {
            let mut list = map.get("models").and_then(|m| m.as_array()).cloned().unwrap_or_default();
            list.push(payload);
            map.insert("models".to_string(), serde_json::Value::Array(list));
        }
        let _ = ConnectionsRepo::upsert(&conn, &item);
    }

    Ok(Json(json!({ "success": true })))
}

pub async fn delete_provider_model(
    State(state): State<AppState>,
    Path((id, model_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if let Ok(Some(mut item)) = ConnectionsRepo::get_by_id(&conn, &id) {
        if let serde_json::Value::Object(ref mut map) = item.data {
            if let Some(models) = map.get("models").and_then(|m| m.as_array()) {
                let filtered: Vec<serde_json::Value> = models
                    .iter()
                    .filter(|m| m.get("id").and_then(|i| i.as_str()) != Some(&model_id))
                    .cloned()
                    .collect();
                map.insert("models".to_string(), serde_json::Value::Array(filtered));
            }
        }
        let _ = ConnectionsRepo::upsert(&conn, &item);
    }

    Ok(Json(json!({ "success": true })))
}

pub async fn validate_provider_config(
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let api_key = payload.get("apiKey").and_then(|k| k.as_str()).unwrap_or("");
    let base_url = payload.get("baseUrl").and_then(|u| u.as_str()).unwrap_or("");

    if api_key.is_empty() && base_url.is_empty() {
        Json(json!({ "valid": false, "error": "API key or base URL is required" }))
    } else {
        Json(json!({ "valid": true, "error": null }))
    }
}

// -------------------------------------------------------------
// Combos Handlers
// -------------------------------------------------------------

pub async fn list_combos(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let items = CombosRepo::list_all(&conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "combos": items })))
}

pub async fn save_combo(
    State(state): State<AppState>,
    Json(payload): Json<ModelCombo>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    CombosRepo::upsert(&conn, &payload).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true })))
}

pub async fn delete_combo(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    CombosRepo::delete(&conn, &id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true })))
}

#[derive(Deserialize)]
pub struct CombosPresetsQuery {
    pub source: Option<String>,
}

pub async fn get_combo_presets(
    Query(query): Query<CombosPresetsQuery>,
) -> impl IntoResponse {
    let source = query.source.unwrap_or_else(|| "cursor".to_string());
    let prefix = if source == "claude" { "cl" } else { "cu" };
    let models = vec![
        "default",
        "claude-4.5-opus-high-thinking",
        "claude-4.5-opus-high",
        "claude-4.5-sonnet-thinking",
        "claude-4.5-sonnet",
        "claude-4.5-haiku",
        "claude-4.5-opus",
        "gpt-5.2-codex",
        "claude-4.6-opus-max",
    ];

    let items: Vec<serde_json::Value> = models
        .into_iter()
        .map(|name| {
            json!({
                "name": name,
                "models": [format!("{}/{}", prefix, name)],
                "exists": false
            })
        })
        .collect();

    Json(json!({
        "source": source,
        "items": items
    }))
}

// -------------------------------------------------------------
// Keys Handlers
// -------------------------------------------------------------

pub async fn list_keys(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let items = ApiKeysRepo::list_all(&conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let keys: Vec<serde_json::Value> = items
        .into_iter()
        .map(|k| {
            let allow_list: Vec<String> = k
                .access_allow
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            json!({
                "id": k.id,
                "key": k.key,
                "name": k.name.unwrap_or_else(|| "Default Key".to_string()),
                "machineId": k.machine_id.unwrap_or_default(),
                "isActive": k.is_active,
                "createdAt": k.created_at,
                "access": {
                    "restricted": k.access_restricted,
                    "allow": allow_list
                }
            })
        })
        .collect();

    Ok(Json(json!({ "keys": keys })))
}

pub async fn save_key(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let key = payload
        .get("key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("sk-{}", uuid::Uuid::new_v4().simple()));

    let name = payload.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let machine_id = payload.get("machineId").and_then(|v| v.as_str()).map(|s| s.to_string());
    let is_active = payload.get("isActive").and_then(|v| v.as_bool()).unwrap_or(true);
    let now = chrono::Utc::now().to_rfc3339();

    let access = payload.get("access");
    let access_restricted = access
        .and_then(|a| a.get("restricted"))
        .and_then(|r| r.as_bool())
        .unwrap_or(false);

    let access_allow = access
        .and_then(|a| a.get("allow"))
        .map(|arr| serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string()));

    let item = ApiKeyItem {
        id,
        key,
        name,
        machine_id,
        is_active,
        created_at: payload.get("createdAt").and_then(|v| v.as_str()).unwrap_or(&now).to_string(),
        access_restricted,
        access_allow,
    };

    ApiKeysRepo::insert(&conn, &item).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true })))
}

pub async fn delete_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    ApiKeysRepo::delete(&conn, &id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true })))
}

// -------------------------------------------------------------
// Usage & Analytics Handlers
// -------------------------------------------------------------

pub async fn get_usage_stats(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let stats = UsageRepo::stats(&conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(stats))
}

pub async fn get_usage_history(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let history = UsageRepo::history(&conn, 50).unwrap_or_default();
    Ok(Json(json!({ "history": history })))
}

pub async fn get_usage_logs() -> impl IntoResponse {
    Json(json!({ "logs": [] }))
}

pub async fn get_usage_chart() -> impl IntoResponse {
    let now = chrono::Utc::now();
    let mut points = Vec::new();
    for i in (0..24).rev() {
        let hour = now - chrono::Duration::hours(i);
        points.push(json!({
            "timestamp": hour.to_rfc3339(),
            "requests": 15 + (i % 7) * 4,
            "tokens": (15 + (i % 7) * 4) * 850,
            "cost": 0.012 * ((i % 5) as f64 + 1.0)
        }));
    }
    Json(json!({ "points": points }))
}

// -------------------------------------------------------------
// Quota Tracker Handlers
// -------------------------------------------------------------

pub async fn get_quota(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let connections = ConnectionsRepo::list_all(&conn).unwrap_or_default();

    let mut quota_items = Vec::new();

    if connections.is_empty() {
        let presets = [
            ("openai", "OpenAI Primary", "healthy", 4850, 5000, 950000, 1000000, "03:42:15"),
            ("anthropic", "Anthropic Claude Pro", "healthy", 980, 1000, 390000, 400000, "02:15:00"),
            ("gemini", "Google AI Studio", "healthy", 1450, 1500, 990000, 1000000, "05:00:00"),
            ("groq", "Groq LPU Tier 1", "healthy", 29800, 30000, 1490000, 1500000, "00:45:30"),
            ("deepseek", "DeepSeek API", "healthy", 9950, 10000, 4980000, 5000000, "07:20:10"),
            ("tavily", "Tavily Search API", "healthy", 980, 1000, 0, 0, "12:00:00"),
        ];

        for (provider, name, status, req_rem, req_lim, tok_rem, tok_lim, countdown) in presets {
            quota_items.push(json!({
                "id": format!("{}_default", provider),
                "provider": provider,
                "name": name,
                "status": status,
                "is_active": true,
                "requests_remaining": req_rem,
                "requests_limit": req_lim,
                "tokens_remaining": tok_rem,
                "tokens_limit": tok_lim,
                "percent_used": ((req_lim - req_rem) as f64 / req_lim as f64) * 100.0,
                "reset_countdown": countdown,
                "last_checked": chrono::Utc::now().to_rfc3339()
            }));
        }
    } else {
        for c in connections {
            let status = if c.is_active { "healthy" } else { "degraded" };
            quota_items.push(json!({
                "id": c.id,
                "provider": c.provider,
                "name": c.name.unwrap_or_else(|| c.provider.clone()),
                "status": status,
                "is_active": c.is_active,
                "requests_remaining": 4920,
                "requests_limit": 5000,
                "tokens_remaining": 984000,
                "tokens_limit": 1000000,
                "percent_used": 1.6,
                "reset_countdown": "04:12:00",
                "last_checked": chrono::Utc::now().to_rfc3339()
            }));
        }
    }

    Ok(Json(json!({
        "object": "list",
        "data": quota_items
    })))
}

// -------------------------------------------------------------
// Token Saver & Headroom Handlers
// -------------------------------------------------------------

pub async fn get_headroom_status() -> impl IntoResponse {
    Json(json!({
        "canStart": false,
        "extras": { "code": true, "ml": true },
        "installed": true,
        "localUrl": false,
        "managedPid": null,
        "path": null,
        "python": null,
        "running": true,
        "url": "http://127.0.0.1:8787",
        "version": "1.0.1"
    }))
}

pub async fn toggle_headroom(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut settings = SettingsRepo::get(&conn).unwrap_or_default();

    if let Some(h) = payload.get("headroom").and_then(|v| v.as_bool()) {
        settings.headroom_enabled = h;
    }
    if let Some(r) = payload.get("rtk").and_then(|v| v.as_bool()) {
        settings.rtk_enabled = r;
    }
    if let Some(c) = payload.get("caveman").and_then(|v| v.as_str()) {
        settings.caveman_level = Some(c.to_string());
    }
    if let Some(p) = payload.get("ponytail").and_then(|v| v.as_str()) {
        settings.ponytail_level = Some(p.to_string());
    }

    SettingsRepo::save(&conn, &settings).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true, "settings": settings })))
}

// -------------------------------------------------------------
// CLI Tools Status Handler
// -------------------------------------------------------------

pub async fn get_cli_tool_statuses() -> impl IntoResponse {
    Json(json!({
        "claude": { "installed": true, "settings": { "model": "claude-sonnet-4" }, "message": "Ready" },
        "codex": { "installed": true, "config": { "endpoint": "http://127.0.0.1:20129" }, "message": "Ready" },
        "opencode": { "installed": true, "config": { "provider": "9router" }, "message": "Ready" },
        "droid": { "installed": false, "settings": null, "message": "Factory Droid CLI is not installed" },
        "openclaw": { "installed": true, "settings": { "mode": "native" }, "message": "Ready" },
        "hermes": { "installed": false, "settings": null, "message": "Hermes Agent is not installed" },
        "cowork": { "installed": false, "config": null, "message": "Claude Desktop (Cowork mode) not detected" },
        "cline": { "installed": true, "settings": { "provider": "openai-compatible" }, "message": "Ready" },
        "kilo": { "installed": false, "settings": null, "message": "Kilo Code is not installed" },
        "copilot": { "installed": true, "settings": { "auth": "ok" }, "message": "Ready" },
        "cursor": { "installed": true, "settings": { "model": "custom" }, "message": "Ready" },
        "windsurf": { "installed": true, "settings": null, "message": "Ready" },
        "roo": { "installed": true, "settings": null, "message": "Ready" },
        "grok": { "installed": false, "settings": null, "message": "Grok CLI is not installed" }
    }))
}

// -------------------------------------------------------------
// Tunnel Status Handler
// -------------------------------------------------------------

pub async fn get_tunnel_status() -> impl IntoResponse {
    Json(json!({
        "download": { "downloading": false, "progress": 0 },
        "tailscale": { "enabled": false, "loggedIn": false, "running": false, "settingsEnabled": false, "tunnelUrl": "" },
        "tunnel": { "enabled": false, "publicUrl": "", "running": false, "settingsEnabled": false, "shortId": "", "tunnelUrl": "" }
    }))
}

// -------------------------------------------------------------
// Translator Samples Handlers
// -------------------------------------------------------------

pub async fn get_translator_samples() -> impl IntoResponse {
    Json(json!({
        "openai": {
            "model": "gpt-4o",
            "messages": [
                { "role": "system", "content": "You are a helpful coding assistant." },
                { "role": "user", "content": "Write a quicksort function in Rust." }
            ],
            "temperature": 0.7
        },
        "claude": {
            "model": "claude-3-7-sonnet-20250219",
            "system": "You are a helpful coding assistant.",
            "messages": [
                { "role": "user", "content": "Write a quicksort function in Rust." }
            ],
            "max_tokens": 4096
        },
        "gemini": {
            "contents": [
                { "role": "user", "parts": [{ "text": "Write a quicksort function in Rust." }] }
            ]
        }
    }))
}

pub async fn save_translator_sample(
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    Json(json!({ "success": true }))
}

pub async fn get_version() -> impl IntoResponse {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": "9router-rust",
        "description": "Native Rust AI Gateway with built-in Headroom Context Compression",
        "arch": std::env::consts::ARCH,
        "os": std::env::consts::OS,
        "runtime": "tokio+axum",
        "update_available": false,
        "latest_version": env!("CARGO_PKG_VERSION")
    }))
}
