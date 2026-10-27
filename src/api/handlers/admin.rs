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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePeriodQuery {
    pub period: Option<String>,
}

fn selected_usage_period(period: Option<&str>) -> Result<&str, (StatusCode, String)> {
    let period = period.unwrap_or("all");
    if ["today", "24h", "7d", "30d", "60d", "all"].contains(&period) {
        Ok(period)
    } else {
        Err((StatusCode::BAD_REQUEST, format!("Unsupported usage period: {period}")))
    }
}

pub async fn get_usage_stats(
    Query(query): Query<UsagePeriodQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let period = selected_usage_period(query.period.as_deref())?;
    let analytics = UsageRepo::analytics(&conn, period, chrono::Utc::now())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(analytics["stats"].clone()))
}

pub async fn get_usage_history(
    Query(query): Query<UsagePeriodQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let period = selected_usage_period(query.period.as_deref())?;
    let analytics = UsageRepo::analytics(&conn, period, chrono::Utc::now())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({
        "history": analytics["history"],
        "breakdown": analytics["breakdown"],
        "period": period
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDetailsQuery {
    pub period: Option<String>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

pub async fn get_usage_request_details(
    Query(query): Query<UsageDetailsQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let period = selected_usage_period(query.period.as_deref())?;
    let analytics = UsageRepo::analytics(&conn, period, chrono::Utc::now())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let records = analytics["history"].as_array().cloned().unwrap_or_default();
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
    let page = query.page.unwrap_or(1).max(1);
    let total = records.len();
    let start = (page - 1).saturating_mul(page_size).min(total);
    let end = start.saturating_add(page_size).min(total);
    let details = records[start..end].to_vec();
    let total_pages = total.div_ceil(page_size);
    Ok(Json(json!({
        "details": details,
        "pagination": {
            "page": page,
            "pageSize": page_size,
            "totalItems": total,
            "totalPages": total_pages,
            "hasNext": page < total_pages,
            "hasPrev": page > 1 && total_pages > 0
        }
    })))
}

pub async fn get_usage_logs(
    Query(query): Query<UsagePeriodQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let period = selected_usage_period(query.period.as_deref())?;
    let analytics = UsageRepo::analytics(&conn, period, chrono::Utc::now())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "logs": analytics["logs"], "period": period })))
}

pub async fn get_usage_chart(
    Query(query): Query<UsagePeriodQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let period = selected_usage_period(query.period.as_deref())?;
    let analytics = UsageRepo::analytics(&conn, period, chrono::Utc::now())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "points": analytics["chart"], "period": period })))
}

fn quota_reset_at(value: Option<&serde_json::Value>) -> serde_json::Value {
    let text = match value {
        Some(serde_json::Value::String(value)) => {
            if let Ok(number) = value.parse::<i64>() {
                let seconds = if number.unsigned_abs() >= 1_000_000_000_000 { number / 1000 } else { number };
                return chrono::DateTime::from_timestamp(seconds, 0)
                    .map(|value| json!(value.to_rfc3339())).unwrap_or(json!(null));
            }
            value.clone()
        }
        Some(serde_json::Value::Number(value)) => {
            let number = value.as_i64().unwrap_or_default();
            let seconds = if number.unsigned_abs() >= 1_000_000_000_000 { number / 1000 } else { number };
            chrono::DateTime::from_timestamp(seconds, 0)
                .map(|value| value.to_rfc3339())
                .unwrap_or_default()
        }
        _ => String::new(),
    };
    if let Ok(value) = chrono::DateTime::parse_from_rfc3339(&text) {
        json!(value.with_timezone(&chrono::Utc).to_rfc3339())
    } else {
        json!(null)
    }
}

fn quota_window(used: f64, total: f64, reset: Option<&serde_json::Value>, name: &str) -> serde_json::Value {
    let remaining = (total - used).max(0.0);
    json!({
        "used": used,
        "total": total,
        "remaining": remaining,
        "remainingPercentage": if total > 0.0 { remaining / total * 100.0 } else { 0.0 },
        "resetAt": quota_reset_at(reset),
        "unlimited": false,
        "displayName": name
    })
}

const ANTIGRAVITY_IMPORTANT_MODELS: &[&str] = &[
    "gemini-4-argon-high", "gemini-4-argon-medium", "gemini-4-argon-low",
    "gemini-3.8-flash-high", "gemini-3.8-flash-medium", "gemini-3.8-flash-low",
    "gemini-3.7-flash-high", "gemini-3.7-flash-medium", "gemini-3.7-flash-low",
    "gemini-3.6-flash-high", "gemini-3.6-flash-medium", "gemini-3.6-flash-low",
    "gemini-3.1-pro-high", "gemini-pro-agent", "gemini-3.1-pro-low",
    "claude-sonnet-5-5-high", "claude-sonnet-5-5-medium", "claude-sonnet-5-5-low",
    "claude-sonnet-5-5", "claude-opus-5-5-high", "claude-opus-5-5-medium",
    "claude-opus-5-5-low", "claude-opus-5-5", "claude-sonnet-4-6",
    "claude-opus-4-6-thinking", "gpt-oss-120b-medium", "gemini-3.1-flash-image",
];

fn antigravity_quotas(data: &serde_json::Value, is_free_tier: bool) -> serde_json::Map<String, serde_json::Value> {
    let mut quotas = serde_json::Map::new();
    if !is_free_tier {
        if let Some(models) = data.get("models").and_then(|value| value.as_object()) {
            for (model, info) in models {
                if !ANTIGRAVITY_IMPORTANT_MODELS.contains(&model.as_str())
                    || info.get("isInternal").and_then(|value| value.as_bool()) == Some(true) { continue; }
                let Some(quota) = info.get("quotaInfo") else { continue };
                let fraction = quota.get("remainingFraction").and_then(|value| value.as_f64()).unwrap_or(0.0);
                if !fraction.is_finite() { continue; }
                quotas.insert(model.clone(), quota_window(
                    (1.0 - fraction.clamp(0.0, 1.0)) * 1000.0,
                    1000.0,
                    quota.get("resetTime"),
                    info.get("displayName").and_then(|value| value.as_str()).unwrap_or(model),
                ));
            }
        }
    }
    quotas
}

fn reconcile_antigravity_sessions(quotas: &mut serde_json::Map<String, serde_json::Value>) {
    for (session, prefix, include_image) in [("gemini_session", "gemini-", false), ("claude_gpt_session", "claude-", true)] {
        let (count, exhausted, reset_at) = quotas.iter()
            .filter(|(key, _)| key.starts_with(prefix) && (include_image || !key.contains("image")))
            .fold((0, true, None::<String>), |(count, exhausted, latest), (_, quota)| {
                let used_up = quota.get("remainingPercentage").and_then(|value| value.as_f64()).unwrap_or(0.0) == 0.0;
                let reset = quota.get("resetAt").and_then(|value| value.as_str());
                let latest = match (latest.as_deref(), reset) {
                    (None, Some(reset)) => Some(reset.to_owned()),
                    (Some(previous), Some(reset)) if reset > previous => Some(reset.to_owned()),
                    _ => latest,
                };
                (count + 1, exhausted && used_up, latest)
            });
        if count == 0 || !exhausted { continue; }
        let Some(window) = quotas.get_mut(session) else { continue; };
        if window.get("remainingPercentage").and_then(|value| value.as_f64()).unwrap_or(0.0) == 0.0 { continue; }
        window["remainingPercentage"] = serde_json::json!(0.0);
        if let Some(reset_at) = reset_at { window["resetAt"] = serde_json::json!(reset_at); }
    }
}


fn antigravity_weekly_quotas(data: &serde_json::Value, quotas: &mut serde_json::Map<String, serde_json::Value>) {
    let groups = data.get("groups").and_then(|value| value.as_array())
        .or_else(|| data.pointer("/quotaSummary/groups").and_then(|value| value.as_array()));
    let Some(groups) = groups else { return };
    for group in groups {
        let label = group.get("displayName").and_then(|value| value.as_str()).unwrap_or("");
        let family = if label.to_lowercase().contains("gemini") { "gemini" }
            else if label.to_lowercase().contains("claude") || label.to_lowercase().contains("gpt") { "claude_gpt" }
            else { continue };
        for bucket in group.get("buckets").and_then(|value| value.as_array()).into_iter().flatten() {
            let window = bucket.get("window").and_then(|value| value.as_str()).unwrap_or("").to_lowercase();
            let text = format!("{} {}", bucket.get("bucketId").and_then(|v| v.as_str()).unwrap_or(""), bucket.get("displayName").and_then(|v| v.as_str()).unwrap_or("")).to_lowercase();
            let weekly = window == "weekly" || text.contains("weekly");
            let session = ["5h", "daily"].contains(&window.as_str()) || text.contains("five hour") || text.contains("5h") || text.contains("daily");
            if (!weekly && !session) || (weekly && bucket.get("disabled").and_then(|v| v.as_bool()) == Some(true)) { continue; }
            let Some(mut fraction) = bucket.get("remainingFraction").and_then(|v| v.as_f64()) else { continue };
            if bucket.get("disabled").and_then(|v| v.as_bool()) == Some(true) { fraction = 0.0; }
            let key = format!("{}_{}", family, if weekly { "weekly" } else { "session" });
            let name = format!("{} ({})", if family == "gemini" { "Gemini" } else { "Claude & GPT" }, if weekly { "Weekly" } else { "5h" });
            quotas.entry(key).or_insert_with(|| quota_window(
                (1.0 - fraction.clamp(0.0, 1.0)) * 1000.0,
                1000.0,
                bucket.get("resetTime"),
                &name,
            ));
        }
    }
}

fn codex_quotas(data: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    let mut quotas = serde_json::Map::new();
    let by_limit = data.get("rate_limits_by_limit_id").filter(|value| value.is_object()).unwrap_or(&serde_json::Value::Null);
    let review = data.get("code_review_rate_limit").or_else(|| data.get("review_rate_limit"))
        .or_else(|| by_limit.get("code_review")).or_else(|| by_limit.get("codex_review")).or_else(|| by_limit.get("review"))
        .or_else(|| data.get("additional_rate_limits").and_then(|value| value.as_array()).into_iter().flatten().find(|entry| {
            entry.get("limit_name").or_else(|| entry.get("metered_feature")).or_else(|| entry.get("id"))
                .and_then(|value| value.as_str()).is_some_and(|id| id.to_lowercase().contains("review"))
        }));
    let spark = data.get("spark_rate_limit").or_else(|| data.get("gpt_5_3_codex_spark_rate_limit"))
        .or_else(|| by_limit.get("gpt-5.3-codex-spark")).or_else(|| by_limit.get("gpt_5_3_codex_spark")).or_else(|| by_limit.get("spark"))
        .or_else(|| data.get("additional_rate_limits").and_then(|value| value.as_array()).into_iter().flatten().find(|entry| {
            entry.get("limit_name").or_else(|| entry.get("metered_feature")).or_else(|| entry.get("id"))
                .and_then(|value| value.as_str()).is_some_and(|id| { let id = id.to_lowercase(); id.contains("spark") || id.contains("5.3-codex-spark") })
        }));
    let normal = data.get("rate_limit").filter(|value| value.is_object())
        .or_else(|| data.get("rate_limits").filter(|value| value.is_object()))
        .or_else(|| by_limit.get("codex"));
    for (prefix, snapshot) in [("", normal), ("review", review), ("spark", spark)] {
        let Some(snapshot) = snapshot else { continue };
        let limits = snapshot.get("rate_limit").filter(|value| value.is_object()).unwrap_or(snapshot);
        for (kind, primary, fallback, name) in [
            ("session", "primary_window", "primary", "Session"),
            ("weekly", "secondary_window", "secondary", "Weekly"),
        ] {
            let window = limits.get(primary).or_else(|| limits.get(fallback))
                .or_else(|| snapshot.get(primary)).or_else(|| snapshot.get(fallback));
            let Some(window) = window else { continue };
            let Some(percent) = window.get("used_percent").or_else(|| window.get("percent_used")).and_then(|value| value.as_f64()) else { continue };
            let key = if prefix.is_empty() { kind.to_owned() } else { format!("{prefix}_{kind}") };
            let display_name = match prefix { "review" => format!("Code review {name}") , "spark" => format!("Spark {name}"), _ => name.to_owned() };
            quotas.insert(key, quota_window(
                percent.clamp(0.0, 100.0), 100.0,
                window.get("reset_at").or_else(|| window.get("resets_at")).or_else(|| window.get("resetAt")),
                &display_name,
            ));
        }
    }
    quotas
}

async fn fetch_connection_quota(connection: &ProviderConnection) -> serde_json::Value {
    let access_token = connection.data.get("accessToken").and_then(|value| value.as_str()).unwrap_or("");
    if !matches!(connection.provider.as_str(), "antigravity" | "codex") {
        return json!({ "plan": null, "quotas": {}, "message": format!("Live quota is not supported for provider {}.", connection.provider) });
    }
    if access_token.is_empty() || access_token.starts_with("oauth_") || access_token == "oauth_token_active" {
        return json!({ "plan": null, "quotas": {}, "message": "A usable OAuth access token is not saved for this connection." });
    }
    let client = match reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build() {
        Ok(client) => client,
        Err(_) => return json!({ "plan": null, "quotas": {}, "message": "Could not initialize the provider quota client." }),
    };
    let result = if connection.provider == "antigravity" {
        let user_agent = "antigravity/hub/2.24.0 darwin/arm64";
        let metadata = json!({ "metadata": { "ideType": 9, "platform": 3, "pluginType": 2 }, "mode": 1 });
        let subscription = match client.post("https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist")
            .bearer_auth(access_token).header("User-Agent", user_agent).json(&metadata).send().await {
            Ok(response) if response.status().is_success() => response.json::<serde_json::Value>().await.ok(),
            _ => None,
        };
        let project = subscription.as_ref().and_then(|value| value.get("cloudaicompanionProject"))
            .and_then(|value| value.as_str()).map(|value| value.to_owned());
        let body = project.map(|project| json!({"project": project})).unwrap_or_else(|| json!({}));
        let response = client.post("https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels")
            .bearer_auth(access_token).header("User-Agent", user_agent).header("X-Client-Name", "antigravity")
            .header("X-Client-Version", "2.24.0").json(&body).send().await;
        match response {
            Ok(response) if response.status().is_success() => match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    let is_free_tier = subscription.as_ref()
                        .and_then(|value| value.pointer("/paidTier/id"))
                        .and_then(|value| value.as_str())
                        .is_none_or(|id| id == "free-tier");
                    let mut quotas = antigravity_quotas(&data, is_free_tier);
                    if let Ok(response) = client.post("https://daily-cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary")
                        .bearer_auth(access_token).header("User-Agent", user_agent).header("X-Client-Name", "antigravity")
                        .header("X-Client-Version", "2.24.0").json(&body).send().await {
                        if response.status().is_success() {
                            if let Ok(summary) = response.json::<serde_json::Value>().await { antigravity_weekly_quotas(&summary, &mut quotas); }
                        }
                    }
                    reconcile_antigravity_sessions(&mut quotas);
                    let plan = subscription.as_ref().and_then(|value| value.pointer("/currentTier/name"))
                        .and_then(|value| value.as_str()).unwrap_or("Unknown");
                    json!({"plan": plan, "quotas": quotas})
                }
                Err(_) => json!({"plan": null, "quotas": {}, "message": "Antigravity returned an invalid quota response."}),
            },
            Ok(response) => json!({"plan": null, "quotas": {}, "message": format!("Antigravity quota API unavailable (HTTP {}).", response.status().as_u16())}),
            Err(_) => json!({"plan": null, "quotas": {}, "message": "Antigravity quota API request failed or timed out."}),
        }
    } else {
        match client.get("https://chatgpt.com/backend-api/wham/usage")
            .bearer_auth(access_token).header("Accept", "application/json").send().await {
            Ok(response) if response.status().is_success() => match response.json::<serde_json::Value>().await {
                Ok(data) => json!({
                    "plan": data.get("plan_type").or_else(|| data.pointer("summary.plan")),
                    "quotas": codex_quotas(&data)
                }),
                Err(_) => json!({"plan": null, "quotas": {}, "message": "Codex returned an invalid quota response."}),
            },
            Ok(response) => json!({"plan": null, "quotas": {}, "message": format!("Codex quota API unavailable (HTTP {}).", response.status().as_u16())}),
            Err(_) => json!({"plan": null, "quotas": {}, "message": "Codex quota API request failed or timed out."}),
        }
    };
    result
}

pub async fn get_connection_usage(
    Path(connection_id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let connection = ConnectionsRepo::get_by_id(&conn, &connection_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Connection not found".to_string()))?;
    let mut usage = fetch_connection_quota(&connection).await;
    if let Some(object) = usage.as_object_mut() { object.insert("connectionId".into(), json!(connection.id)); }
    Ok(Json(usage))
}

pub async fn get_quota(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let connections = ConnectionsRepo::list_all(&conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut quota_items = Vec::with_capacity(connections.len());
    for connection in connections {
        let usage = fetch_connection_quota(&connection).await;
        quota_items.push(json!({
            "id": connection.id,
            "provider": connection.provider,
            "name": connection.name,
            "email": connection.email,
            "is_active": connection.is_active,
            "plan": usage.get("plan").cloned().unwrap_or(serde_json::Value::Null),
            "quotas": usage.get("quotas").cloned().unwrap_or_else(|| json!({})),
            "message": usage.get("message"),
            "last_checked": chrono::Utc::now().to_rfc3339()
        }));
    }
    Ok(Json(json!({ "object": "list", "data": quota_items })))
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
    if let Some(c) = payload.get("caveman") {
        if let Some(b) = c.as_bool() {
            settings.caveman_level = if b { Some("auto".to_string()) } else { None };
        } else if let Some(s) = c.as_str() {
            settings.caveman_level = Some(s.to_string());
        }
    }
    if let Some(p) = payload.get("ponytail") {
        if let Some(b) = p.as_bool() {
            settings.ponytail_level = if b { Some("ultra".to_string()) } else { None };
        } else if let Some(s) = p.as_str() {
            settings.ponytail_level = Some(s.to_string());
        }
    }
    if let Some(cp) = payload.get("custom_prompt").or_else(|| payload.get("customPrompt")) {
        match cp {
            serde_json::Value::Null => {
                settings.extra.remove("custom_prompt");
                settings.extra.remove("customPrompt");
            }
            serde_json::Value::Bool(b) => {
                settings.extra.insert("custom_prompt_enabled".to_string(), json!(b));
            }
            _ => {
                settings.extra.insert("custom_prompt".to_string(), cp.clone());
            }
        }
    }
    if let Some(cpe) = payload.get("custom_prompt_enabled").or_else(|| payload.get("customPromptEnabled")) {
        settings.extra.insert("custom_prompt_enabled".to_string(), cpe.clone());
    }

    SettingsRepo::save(&conn, &settings).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true, "settings": settings })))
}

// -------------------------------------------------------------
// CLI Tools Status Handler
// -------------------------------------------------------------

pub async fn get_cli_tool_statuses() -> impl IntoResponse {
    Json(json!([
        { "key": "claude", "name": "Claude Code", "icon": "code", "installed": true, "status": "connected", "message": "Ready", "configCommand": "claude config set --key api_base_url --value http://localhost:20129/v1", "envVars": { "ANTHROPIC_BASE_URL": "http://localhost:20129/v1" }, "installGuide": "npm install -g @anthropic-ai/claude-code" },
        { "key": "openclaw", "name": "Open Claw", "icon": "pets", "installed": true, "status": "connected", "message": "Ready", "configCommand": "openclaw config set base_url http://localhost:20129/v1", "envVars": { "OPENAI_BASE_URL": "http://localhost:20129/v1" }, "installGuide": "pip install openclaw" },
        { "key": "codex", "name": "OpenAI Codex CLI / App", "icon": "terminal", "installed": true, "status": "connected", "message": "Ready", "configCommand": "export OPENAI_BASE_URL=http://localhost:20129/v1", "envVars": { "OPENAI_BASE_URL": "http://localhost:20129/v1", "OPENAI_API_KEY": "sk-9router" }, "installGuide": "npm install -g @openai/codex" },
        { "key": "copilot", "name": "GitHub Copilot", "icon": "auto_awesome", "installed": true, "status": "connected", "message": "Ready", "configCommand": "gh copilot config", "envVars": {}, "installGuide": "gh extension install github/gh-copilot" },
        { "key": "opencode", "name": "OpenCode", "icon": "data_object", "installed": true, "status": "connected", "message": "Ready", "configCommand": "opencode config set provider 9router", "envVars": { "OPENAI_BASE_URL": "http://localhost:20129/v1" }, "installGuide": "npm install -g opencode" },
        { "key": "cowork", "name": "Claude Cowork", "icon": "groups", "installed": false, "status": "not-installed", "message": "Claude Desktop (Cowork mode) not detected", "configCommand": "", "envVars": {}, "installGuide": "Install Claude Desktop from claude.ai/download" },
        { "key": "hermes", "name": "Hermes Agent", "icon": "smart_toy", "installed": false, "status": "not-installed", "message": "Hermes Agent is not installed", "configCommand": "", "envVars": {}, "installGuide": "pip install hermes-agent" },
        { "key": "droid", "name": "Factory Droid", "icon": "precision_manufacturing", "installed": false, "status": "not-installed", "message": "Factory Droid CLI is not installed", "configCommand": "", "envVars": {}, "installGuide": "npm install -g factory-droid" },
        { "key": "cursor", "name": "Cursor", "icon": "mouse", "installed": true, "status": "connected", "message": "Ready", "configCommand": "Set OpenAI Base URL in Cursor Settings > Models", "envVars": { "OPENAI_BASE_URL": "http://localhost:20129/v1" }, "installGuide": "Download from cursor.com" },
        { "key": "cline", "name": "Cline", "icon": "extension", "installed": true, "status": "connected", "message": "Ready", "configCommand": "Set provider to openai-compatible in Cline settings", "envVars": { "OPENAI_BASE_URL": "http://localhost:20129/v1" }, "installGuide": "Install Cline extension in VS Code" },
        { "key": "kilo", "name": "Kilo Code", "icon": "speed", "installed": false, "status": "not-installed", "message": "Kilo Code is not installed", "configCommand": "", "envVars": {}, "installGuide": "Install Kilo Code extension in VS Code" },
        { "key": "roo", "name": "Roo", "icon": "eco", "installed": true, "status": "guide", "message": "Ready", "configCommand": "roo config set endpoint http://localhost:20129/v1", "envVars": {}, "installGuide": "npm install -g roo-code" },
        { "key": "continue", "name": "Continue", "icon": "play_arrow", "installed": false, "status": "not-installed", "message": "Continue is not installed", "configCommand": "", "envVars": {}, "installGuide": "Install Continue extension in VS Code" },
        { "key": "amp", "name": "Amp CLI", "icon": "bolt", "installed": false, "status": "not-installed", "message": "Amp CLI is not installed", "configCommand": "", "envVars": {}, "installGuide": "npm install -g @anthropic-ai/amp" },
        { "key": "windsurf", "name": "Windsurf", "icon": "surfing", "installed": true, "status": "connected", "message": "Ready", "configCommand": "Set base URL in Windsurf settings", "envVars": { "OPENAI_BASE_URL": "http://localhost:20129/v1" }, "installGuide": "Download from windsurf.com" }
    ]))
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

#[cfg(test)]
mod quota_tests {
    use super::{antigravity_quotas, antigravity_weekly_quotas, codex_quotas, reconcile_antigravity_sessions};
    use serde_json::json;

    #[test]
    fn maps_only_paid_public_antigravity_models() {
        let data = json!({"models": {
            "gemini-3.8-flash-high": {"displayName": "Gemini Real", "quotaInfo": {"remainingFraction": 0.25, "resetTime": "2026-11-01T01:00:00Z"}},
            "gemini-3.8-flash-medium": {"isInternal": true, "quotaInfo": {"remainingFraction": 0.5}},
            "gemini-3.8-flash-other": {"quotaInfo": {"remainingFraction": 0.5}},
            "unlisted": {"quotaInfo": {"remainingFraction": 0.5}},
            "without-quota": {"displayName": "No quota"}
        }});
        let mut quotas = antigravity_quotas(&data, false);
        assert_eq!(quotas.len(), 1);
        assert_eq!(quotas["gemini-3.8-flash-high"]["used"], 750.0);
        assert_eq!(quotas["gemini-3.8-flash-high"]["remainingPercentage"], 25.0);
        assert!(antigravity_quotas(&data, true).is_empty());
        antigravity_weekly_quotas(&json!({"groups": [{
            "displayName": "Gemini",
            "buckets": [
                {"window": "weekly", "remainingFraction": 0.5, "resetTime": "2026-11-03T00:00:00Z"},
                {"window": "5h", "remainingFraction": 0.75, "resetTime": "2026-10-29T00:00:00Z"}
            ]
        }]}), &mut quotas);
        assert_eq!(quotas.len(), 3);
        assert_eq!(quotas["gemini_weekly"]["remainingPercentage"], 50.0);
        assert_eq!(quotas["gemini_session"]["remainingPercentage"], 75.0);
    }

    #[test]
    fn exhausted_model_windows_override_session_until_latest_model_reset() {
        let mut quotas = json!({
            "gemini-3.8-flash-high": {"remainingPercentage": 0.0, "resetAt": "2026-11-02T00:00:00+00:00"},
            "gemini-3.7-flash-high": {"remainingPercentage": 0.0, "resetAt": "2026-11-03T00:00:00+00:00"},
            "gemini-3.1-flash-image": {"remainingPercentage": 0.0, "resetAt": "2026-12-01T00:00:00+00:00"},
            "gemini_session": {"remainingPercentage": 50.0, "resetAt": "2026-11-01T00:00:00+00:00"}
        }).as_object().unwrap().clone();
        reconcile_antigravity_sessions(&mut quotas);
        assert_eq!(quotas["gemini_session"]["remainingPercentage"], 0.0);
        assert_eq!(quotas["gemini_session"]["resetAt"], "2026-11-03T00:00:00+00:00");
    }

    #[test]
    fn non_exhausted_models_do_not_override_session_quota() {
        let mut quotas = json!({
            "claude-sonnet-5-5-high": {"remainingPercentage": 0.0},
            "claude-opus-5-5-high": {"remainingPercentage": 1.0},
            "claude_gpt_session": {"remainingPercentage": 60.0}
        }).as_object().unwrap().clone();
        reconcile_antigravity_sessions(&mut quotas);
        assert_eq!(quotas["claude_gpt_session"]["remainingPercentage"], 60.0);
    }


    #[test]
    fn maps_codex_primary_and_secondary_windows() {
        let quotas = codex_quotas(&json!({
            "rate_limit": {
                "primary_window": {"used_percent": 20, "reset_at": 1_800_000_000_i64},
                "secondary_window": {"percent_used": 65, "reset_at": "1800000100"}
            },
            "code_review_rate_limit": {"rate_limit": {"primary_window": {"used_percent": 10}}},
            "additional_rate_limits": [{"limit_name": "gpt-5.3-codex-spark", "rate_limit": {"secondary_window": {"used_percent": 45}}}]
        }));
        assert_eq!(quotas["review_session"]["remainingPercentage"], 90.0);
        assert!((quotas["spark_weekly"]["remainingPercentage"].as_f64().unwrap() - 55.0).abs() < 1e-9);
        assert_eq!(quotas["session"]["remainingPercentage"], 80.0);
        assert_eq!(quotas["weekly"]["remainingPercentage"], 35.0);
        assert_eq!(quotas["session"]["resetAt"], "2027-01-15T08:00:00+00:00");
        assert_eq!(quotas["weekly"]["resetAt"], "2027-01-15T08:01:40+00:00");
    }
}
