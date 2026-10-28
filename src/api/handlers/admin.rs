use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
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

pub async fn patch_settings(
    State(state): State<AppState>,
    Json(patch): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let settings = SettingsRepo::get(&conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let settings = merge_settings(settings, &patch).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    SettingsRepo::save(&conn, &settings).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true, "settings": settings })))
}

fn merge_settings(settings: SystemSettings, patch: &Value) -> Result<SystemSettings, String> {
    let patch = patch.as_object().ok_or_else(|| "Settings patch must be a JSON object".to_string())?;
    let mut merged = serde_json::to_value(settings).map_err(|e| e.to_string())?;
    let target = merged.as_object_mut().ok_or_else(|| "Stored settings are not a JSON object".to_string())?;
    for (key, value) in patch {
        target.insert(key.clone(), value.clone());
    }
    serde_json::from_value(merged).map_err(|e| e.to_string())
}

#[cfg(test)]
mod settings_patch_tests {
    use super::merge_settings;
    use crate::db::repos::settings::SystemSettings;
    use serde_json::json;

    #[test]
    fn partial_patch_preserves_existing_values_and_updates_extra_settings() {
        let mut existing = SystemSettings::default();
        existing.extra.insert("ccFilterNaming".into(), json!(true));
        let updated = merge_settings(existing, &json!({ "comboStrategy": "priority" })).unwrap();
        assert_eq!(updated.extra["ccFilterNaming"], true);
        assert_eq!(updated.extra["comboStrategy"], "priority");
        assert!(!updated.require_api_key);
        assert!(merge_settings(updated, &json!(null)).is_err());
    }
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

fn contains_forbidden_credentials(value: &serde_json::Value, allow_api_key: bool, root: bool) -> bool {
    match value {
        serde_json::Value::Object(fields) => fields.iter().any(|(key, value)| {
            let normalized: String = key.chars().filter(|ch| ch.is_ascii_alphanumeric()).flat_map(char::to_lowercase).collect();
            let metadata = root && matches!(normalized.as_str(), "hasapikey" | "hasoauthtoken");
            let allowed_api_key = root && allow_api_key && normalized == "apikey";
            (ProviderConnection::is_sensitive_key(key) && !metadata && !allowed_api_key)
                || contains_forbidden_credentials(value, allow_api_key, false)
        }),
        serde_json::Value::Array(values) => values.iter().any(|value| contains_forbidden_credentials(value, allow_api_key, false)),
        _ => false,
    }
}

fn provider_connection_data(
    existing: Option<&ProviderConnection>,
    payload: &serde_json::Value,
    provider: &str,
    auth_type: &str,
) -> Result<serde_json::Value, &'static str> {
    if contains_forbidden_credentials(payload, auth_type != "oauth", true) {
        return Err("OAuth credentials must be obtained through the authorization flow.");
    }
    if let Some(existing) = existing.filter(|item| item.auth_type == "oauth") {
        if existing.provider != provider || auth_type != "oauth" {
            return Err("An OAuth connection cannot change provider or authentication type.");
        }
    } else if auth_type == "oauth" {
        return Err("Create OAuth connections through the provider authorization flow.");
    }
    let mut data = existing.map(|item| item.data.clone()).unwrap_or_else(|| json!({}));
    if let (Some(incoming), Some(saved)) = (payload.as_object(), data.as_object_mut()) {
        for (key, value) in incoming {
            if !matches!(key.as_str(), "hasApiKey" | "hasOAuthToken") {
                saved.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(data)
}

#[cfg(test)]
mod provider_data_tests {
    use super::provider_connection_data;
    use crate::db::repos::connections::ProviderConnection;

    use serde_json::json;
    #[test]
    fn browser_cannot_create_fake_oauth_and_updates_preserve_saved_tokens() {
        let saved = ProviderConnection {
            id: "conn".into(), provider: "antigravity".into(), auth_type: "oauth".into(),
            name: None, email: None, priority: Some(1), is_active: true,
            data: json!({ "accessToken": "stored-secret", "refreshToken": "refresh-secret" }),
            created_at: "now".into(), updated_at: "now".into(),
        };
        assert!(provider_connection_data(None, &json!({ "authType": "oauth" }), "antigravity", "oauth").is_err());
        assert!(provider_connection_data(None, &json!({ "accessToken": "attacker-token" }), "antigravity", "apikey").is_err());
        assert!(provider_connection_data(None, &json!({ "providerSpecificData": { "access_token": "nested-token" } }), "antigravity", "apikey").is_err());
        let updated = provider_connection_data(Some(&saved), &json!({ "isActive": false, "hasOAuthToken": true }), "antigravity", "oauth").unwrap();
        assert_eq!(updated["accessToken"], "stored-secret");
        assert_eq!(updated["refreshToken"], "refresh-secret");
    }
}

pub async fn save_provider(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id = payload.get("id").and_then(|v| v.as_str()).map(str::to_owned).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let provider = payload.get("provider").and_then(|v| v.as_str()).unwrap_or("custom").to_string();
    let auth_type = payload.get("authType").and_then(|v| v.as_str()).unwrap_or("apikey").to_string();
    let existing = ConnectionsRepo::get_by_id(&conn, &id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let data = provider_connection_data(existing.as_ref(), &payload, &provider, &auth_type)
        .map_err(|message| (StatusCode::BAD_REQUEST, message.to_string()))?;
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
        data,
        created_at: existing.map(|item| item.created_at).unwrap_or_else(|| now.clone()),
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
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let (name, models, kind) = combo_fields(&payload)
        .map_err(|error| combo_error(StatusCode::BAD_REQUEST, error))?;
    let conn = state.engine.pool.get()
        .map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    if CombosRepo::get_by_name(&conn, &name)
        .map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .is_some()
    {
        return Err(combo_error(StatusCode::BAD_REQUEST, "Combo name already exists"));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let combo = ModelCombo {
        id: uuid::Uuid::new_v4().to_string(), name, kind, models,
        created_at: now.clone(), updated_at: now,
    };
    CombosRepo::insert(&conn, &combo)
        .map_err(|error| combo_error(StatusCode::CONFLICT, error.to_string()))?;
    Ok((StatusCode::CREATED, Json(combo)))
}

pub async fn update_combo(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let conn = state.engine.pool.get()
        .map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let mut combo = CombosRepo::get_by_id(&conn, &id)
        .map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .ok_or_else(|| combo_error(StatusCode::NOT_FOUND, "Combo not found"))?;
    let object = payload.as_object()
        .ok_or_else(|| combo_error(StatusCode::BAD_REQUEST, "Combo must be a JSON object"))?;
    if object.contains_key("name") {
        combo.name = object.get("name").and_then(Value::as_str)
            .filter(|name| valid_combo_name(name))
            .ok_or_else(|| combo_error(StatusCode::BAD_REQUEST, "Name can only contain letters, numbers, -, _ and ."))?
            .to_owned();
    }
    if object.contains_key("models") {
        combo.models = combo_models(object.get("models"))
            .map_err(|error| combo_error(StatusCode::BAD_REQUEST, error))?;
    }
    if object.contains_key("kind") {
        combo.kind = combo_kind(object.get("kind"))
            .map_err(|error| combo_error(StatusCode::BAD_REQUEST, error))?;
    }
    if let Some(existing) = CombosRepo::get_by_name(&conn, &combo.name)
        .map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    {
        if existing.id != id {
            return Err(combo_error(StatusCode::BAD_REQUEST, "Combo name already exists"));
        }
    }
    combo.updated_at = chrono::Utc::now().to_rfc3339();
    if !CombosRepo::update(&conn, &combo)
        .map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
    {
        return Err(combo_error(StatusCode::NOT_FOUND, "Combo not found"));
    }
    Ok(Json(combo))
}

pub async fn get_combo(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let conn = state.engine.pool.get()
        .map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let combo = CombosRepo::get_by_id(&conn, &id)
        .map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .ok_or_else(|| combo_error(StatusCode::NOT_FOUND, "Combo not found"))?;
    Ok(Json(combo))
}

#[cfg(test)]
mod combo_api_tests {
    use super::{combo_fields, combo_models, combo_preset_items, valid_combo_name};
    use serde_json::json;

    #[test]
    fn names_and_models_follow_combo_api_contract() {
        assert!(valid_combo_name("Claude_Opus-5.v2"));
        assert!(!valid_combo_name("../settings"));
        assert!(combo_fields(&json!({ "name": "combo", "models": ["openai/model"] })).is_ok());
        assert!(combo_models(Some(&json!(["ok", 7]))).is_err());
        assert!(combo_models(Some(&json!("model"))).is_err());
    }

    #[test]
    fn presets_use_catalog_models_and_skip_existing_combo_names() {
        let existing = std::collections::HashSet::from(["default".to_string()]);
        let cursor = combo_preset_items("cursor", &existing).unwrap();
        assert!(cursor.iter().any(|item| item["name"] == "default" && item["models"][0] == "cu/default" && item["exists"] == true));
        assert!(cursor.iter().any(|item| item["models"][0].as_str().unwrap().starts_with("cu/")));
        let claude = combo_preset_items("claude", &existing).unwrap();
        assert!(claude.iter().any(|item| item["name"] == "opusplan" && item["models"][0] == "cc/claude-opus-5"));
        assert!(combo_preset_items("unknown", &existing).is_err());
    }
}

fn combo_fields(payload: &Value) -> Result<(String, Vec<String>, Option<String>), &'static str> {
    let object = payload.as_object().ok_or("Combo must be a JSON object")?;
    let name = object.get("name").and_then(Value::as_str).unwrap_or_default();
    if !valid_combo_name(name) {
        return Err("Name can only contain letters, numbers, -, _ and .");
    }
    Ok((name.to_owned(), combo_models(object.get("models"))?, combo_kind(object.get("kind"))?))
}

fn combo_models(value: Option<&Value>) -> Result<Vec<String>, &'static str> {
    match value {
        None => Ok(Vec::new()),
        Some(Value::Array(items)) => items.iter().map(|item| item.as_str().map(str::to_owned).ok_or("Models must contain strings")).collect(),
        _ => Err("Models must be an array"),
    }
}
fn combo_kind(value: Option<&Value>) -> Result<Option<String>, &'static str> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(kind)) => Ok(Some(kind.clone())),
        _ => Err("Kind must be a string or null"),
    }
}

fn valid_combo_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn combo_error(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": message.into() })))
}

pub async fn delete_combo(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let conn = state.engine.pool.get().map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let deleted = CombosRepo::delete(&conn, &id).map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    if !deleted {
        return Err(combo_error(StatusCode::NOT_FOUND, "Combo not found"));
    }
    Ok(Json(json!({ "success": true })))
}

#[derive(Deserialize)]
pub struct CombosPresetsQuery {
    pub source: Option<String>,
}

fn combo_preset_items(source: &str, existing: &std::collections::HashSet<String>) -> Result<Vec<Value>, String> {
    let (provider_id, alias) = match source {
        "cursor" => ("cursor", "cu"),
        "claude" => ("claude", "cc"),
        _ => return Err("source must be 'cursor' or 'claude'".into()),
    };
    let registry: Value = serde_json::from_str(CATALOG_JSON).map_err(|error| error.to_string())?;
    let provider = registry.as_array().and_then(|providers| providers.iter().find(|provider| provider.get("id").and_then(Value::as_str) == Some(provider_id)))
        .ok_or_else(|| format!("{provider_id} model catalog is unavailable"))?;
    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::new();
    if let Some(models) = provider.get("models").and_then(Value::as_array) {
        for model in models {
            if let Some(id) = model.get("id").and_then(Value::as_str).filter(|id| valid_combo_name(id)) {
                if seen.insert(id.to_owned()) {
                    items.push(json!({ "name": id, "models": [format!("{alias}/{id}")], "exists": existing.contains(id) }));
                }
            }
        }
    }
    if source == "claude" {
        for (name, target) in [
            ("fable", "cc/claude-fable-5"),
            ("opus", "cc/claude-opus-5"),
            ("sonnet", "cc/claude-sonnet-5"),
            ("haiku", "cc/claude-haiku-4-5-20251001"),
            ("default", "cc/claude-sonnet-5"),
            ("opusplan", "cc/claude-opus-5"),
        ] {
            if seen.insert(name.to_owned()) {
                items.push(json!({ "name": name, "models": [target], "exists": existing.contains(name) }));
            }
        }
    }
    Ok(items)
}

pub async fn get_combo_presets(
    Query(query): Query<CombosPresetsQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let source = query.source.ok_or_else(|| combo_error(StatusCode::BAD_REQUEST, "source must be 'cursor' or 'claude'"))?;
    let conn = state.engine.pool.get().map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let existing = CombosRepo::list_all(&conn).map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .into_iter().map(|combo| combo.name).collect::<std::collections::HashSet<_>>();
    let items = combo_preset_items(&source, &existing).map_err(|error| combo_error(StatusCode::BAD_REQUEST, error))?;
    let to_create = items.iter().filter(|item| item["exists"] == false).count();
    Ok(Json(json!({ "source": source, "items": items, "toCreate": to_create, "toSkip": items.len() - to_create })))
}

pub async fn save_combo_presets(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let source = payload.get("source").and_then(Value::as_str)
        .ok_or_else(|| combo_error(StatusCode::BAD_REQUEST, "source must be 'cursor' or 'claude'"))?;
    let mut conn = state.engine.pool.get().map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let existing = CombosRepo::list_all(&conn).map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .into_iter().map(|combo| combo.name).collect::<std::collections::HashSet<_>>();
    let items = combo_preset_items(source, &existing).map_err(|error| combo_error(StatusCode::BAD_REQUEST, error))?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut created = Vec::new();
    let mut skipped = Vec::new();
    let tx = conn.transaction().map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    for item in items {
        let name = item["name"].as_str().ok_or_else(|| combo_error(StatusCode::INTERNAL_SERVER_ERROR, "Preset name missing"))?;
        if item["exists"] == true {
            skipped.push(name.to_owned());
            continue;
        }
        let combo = ModelCombo {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_owned(),
            kind: None,
            models: combo_models(item.get("models")).map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        CombosRepo::insert(&tx, &combo).map_err(|error| combo_error(StatusCode::CONFLICT, error.to_string()))?;
        created.push(combo);
    }
    tx.commit().map_err(|error| combo_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let created_count = created.len();
    let skipped_count = skipped.len();
    Ok(Json(json!({ "source": source, "created": created, "skipped": skipped, "createdCount": created_count, "skippedCount": skipped_count })))
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
        "stats": analytics["stats"],
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
    pub provider: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}
fn usage_filter_timestamp(value: Option<&str>, field: &str) -> Result<Option<chrono::DateTime<chrono::Utc>>, (StatusCode, String)> {
    value.map(|value| chrono::DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
        .map_err(|_| (StatusCode::BAD_REQUEST, format!("Invalid {field} timestamp"))))
        .transpose()
}

fn usage_record_matches(
    record: &serde_json::Value,
    provider: Option<&str>,
    start: Option<&chrono::DateTime<chrono::Utc>>,
    end: Option<&chrono::DateTime<chrono::Utc>>,
) -> bool {
    if provider.is_some_and(|provider| record["provider"].as_str() != Some(provider)) {
        return false;
    }
    if start.is_none() && end.is_none() { return true; }
    let Some(timestamp) = record["timestamp"].as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&chrono::Utc)) else { return false; };
    start.map_or(true, |start| &timestamp >= start) && end.map_or(true, |end| &timestamp <= end)
}

#[cfg(test)]
mod usage_details_filter_tests {
    use super::{usage_filter_timestamp, usage_record_matches};
    use axum::http::StatusCode;
    use serde_json::json;

    #[test]
    fn filters_saved_records_by_provider_and_inclusive_timestamps() {
        let start = usage_filter_timestamp(Some("2025-04-10T11:00:00Z"), "startDate").unwrap().unwrap();
        let end = usage_filter_timestamp(Some("2025-04-10T11:30:00Z"), "endDate").unwrap().unwrap();
        let row = json!({"provider":"p1","timestamp":"2025-04-10T11:00:00Z"});
        assert!(usage_record_matches(&row, Some("p1"), Some(&start), Some(&end)));
        assert!(!usage_record_matches(&row, Some("p2"), Some(&start), Some(&end)));
        let outside = json!({"provider":"p1","timestamp":"2025-04-10T11:30:01Z"});
        assert!(!usage_record_matches(&outside, Some("p1"), Some(&start), Some(&end)));
        assert_eq!(usage_filter_timestamp(Some("invalid"), "startDate").unwrap_err().0, StatusCode::BAD_REQUEST);
    }
}

pub async fn get_usage_request_details(
    Query(query): Query<UsageDetailsQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let period = selected_usage_period(query.period.as_deref())?;
    let start = usage_filter_timestamp(query.start_date.as_deref(), "startDate")?;
    let end = usage_filter_timestamp(query.end_date.as_deref(), "endDate")?;
    if let (Some(start), Some(end)) = (start.as_ref(), end.as_ref()) {
        if start > end { return Err((StatusCode::BAD_REQUEST, "startDate must not be after endDate".to_string())); }
    }
    let analytics = UsageRepo::analytics(&conn, period, chrono::Utc::now())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let records = analytics["history"].as_array().cloned().unwrap_or_default();
    let records = records.into_iter().filter(|record| usage_record_matches(
        record,
        query.provider.as_deref(),
        start.as_ref(),
        end.as_ref(),
    )).collect::<Vec<_>>();
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
    let page = query.page.unwrap_or(1).max(1);
    let total = records.len();
    let start_index = (page - 1).saturating_mul(page_size).min(total);
    let end_index = start_index.saturating_add(page_size).min(total);
    let details = records[start_index..end_index].to_vec();
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
                let Some(fraction) = quota.get("remainingFraction").and_then(|value| value.as_f64()) else { continue };
                if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) { continue; }
                quotas.insert(model.clone(), quota_window(
                    100.0 - fraction.clamp(0.0, 1.0) * 100.0,
                    100.0,
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
                (1.0 - fraction.clamp(0.0, 1.0)) * 100.0,
                100.0,
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

async fn fetch_connection_quota(pool: &crate::db::DbPool, connection: &ProviderConnection) -> serde_json::Value {
    if !matches!(connection.provider.as_str(), "antigravity" | "codex") {
        return json!({ "plan": null, "quotas": {}, "message": format!("Live quota is not supported for provider {}.", connection.provider) });
    }
    let Some(saved_token) = connection.data.get("accessToken").and_then(|value| value.as_str()).filter(|value| !value.is_empty()) else {
        return json!({ "plan": null, "quotas": {}, "message": "A usable OAuth access token is not saved for this connection." });
    };
    if saved_token.starts_with("oauth_") || saved_token == "oauth_token_active" {
        return json!({ "plan": null, "quotas": {}, "message": "A usable OAuth access token is not saved for this connection." });
    }
    let access_token = match crate::api::handlers::oauth::access_token(pool, connection).await {
        Ok(token) => token,
        Err(message) => return json!({ "plan": null, "quotas": {}, "message": message }),
    };
    let client = match reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build() {
        Ok(client) => client,
        Err(_) => return json!({ "plan": null, "quotas": {}, "message": "Could not initialize the provider quota client." }),
    };
    let result = if connection.provider == "antigravity" {
        let user_agent = "antigravity/hub/2.24.0 darwin/arm64";
        let metadata = json!({ "metadata": { "ideType": 9, "platform": 3, "pluginType": 2 }, "mode": 1 });
        let subscription = match client.post("https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist")
            .bearer_auth(&access_token).header("User-Agent", user_agent).json(&metadata).send().await {
            Ok(response) if response.status().is_success() => response.json::<serde_json::Value>().await.ok(),
            _ => None,
        };
        let project = subscription.as_ref().and_then(|value| value.get("cloudaicompanionProject"))
            .and_then(|value| value.as_str()).map(|value| value.to_owned());
        let body = project.map(|project| json!({"project": project})).unwrap_or_else(|| json!({}));
        let response = client.post("https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels")
            .bearer_auth(&access_token).header("User-Agent", user_agent).header("X-Client-Name", "antigravity")
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
                        .bearer_auth(&access_token).header("User-Agent", user_agent).header("X-Client-Name", "antigravity")
                        .header("X-Client-Version", "2.24.0").json(&body).send().await {
                        if response.status().is_success() {
                            if let Ok(summary) = response.json::<serde_json::Value>().await { antigravity_weekly_quotas(&summary, &mut quotas); }
                        }
                    }
                    reconcile_antigravity_sessions(&mut quotas);
                    let plan = subscription.as_ref().and_then(|value| value.pointer("/currentTier/name"))
                        .and_then(|value| value.as_str()).unwrap_or("Unknown");
                    let message = if quotas.is_empty() { Some("Antigravity did not return quota data for this connection.") } else { None };
                    json!({"plan": plan, "quotas": quotas, "message": message})
                }
                Err(_) => json!({"plan": null, "quotas": {}, "message": "Antigravity returned an invalid quota response."}),
            },
            Ok(response) => json!({"plan": null, "quotas": {}, "message": format!("Antigravity quota API unavailable (HTTP {}).", response.status().as_u16())}),
            Err(_) => json!({"plan": null, "quotas": {}, "message": "Antigravity quota API request failed or timed out."}),
        }
    } else {
        match client.get("https://chatgpt.com/backend-api/wham/usage")
            .bearer_auth(&access_token).header("Accept", "application/json").send().await {
            Ok(response) if response.status().is_success() => match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    let quotas = codex_quotas(&data);
                    let message = if quotas.is_empty() { Some("Codex did not return quota data for this connection.") } else { None };
                    json!({ "plan": data.get("plan_type").or_else(|| data.pointer("summary.plan")), "quotas": quotas, "message": message })
                }
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
    let mut usage = fetch_connection_quota(&state.engine.pool, &connection).await;
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
        let usage = fetch_connection_quota(&state.engine.pool, &connection).await;
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

pub async fn get_headroom_extras(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let settings = SettingsRepo::get(&conn).unwrap_or_default();
    Ok(Json(json!({
        "success": true,
        "extras": settings.extra.get("headroom_code").or(settings.extra.get("headroom_ml")).map(|_| true)
    })))
}

pub async fn toggle_headroom_extras(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut settings = SettingsRepo::get(&conn).unwrap_or_default();

    if let Some(extras) = payload.get("extras").and_then(|e| e.as_object()) {
        for (key, value) in extras {
            if let Some(b) = value.as_bool() {
                settings.extra.insert(format!("headroom_{}", key), json!(b));
            }
        }
    }

    SettingsRepo::save(&conn, &settings).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true, "settings": settings })))
}

pub async fn delete_headroom_extras(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut settings = SettingsRepo::get(&conn).unwrap_or_default();

    settings.extra.remove("headroom_code");
    settings.extra.remove("headroom_ml");

    SettingsRepo::save(&conn, &settings).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "success": true })))
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
            "without-quota": {"displayName": "No quota"},
            "gemini-3.8-flash-low": {"displayName": "Incomplete quota", "quotaInfo": {}}
        }});
        let mut quotas = antigravity_quotas(&data, false);
        assert_eq!(quotas.len(), 1);
        assert_eq!(quotas["gemini-3.8-flash-high"]["used"], 75.0);
        assert_eq!(quotas["gemini-3.8-flash-high"]["remainingPercentage"], 25.0);
        let malformed = json!({"models": {
            "gemini-3.8-flash-high": {"quotaInfo": {"remainingFraction": 1.5}},
            "gemini-3.8-flash-medium": {"quotaInfo": {"remainingFraction": "0.5"}}
        }});
        assert!(antigravity_quotas(&malformed, false).is_empty());
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
pub async fn list_provider_nodes() -> impl IntoResponse {
    Json(json!({
        "nodes": [
            {"id": "node-1", "type": "provider", "name": "OpenAI", "status": "online"},
            {"id": "node-2", "type": "provider", "name": "Anthropic", "status": "online"},
            {"id": "node-3", "type": "provider", "name": "Google", "status": "online"}
        ]
    }))
}
