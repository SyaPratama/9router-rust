use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use router9::api::build_router;
use router9::db::repos::keys::{ApiKeyItem, ApiKeysRepo};
use router9::db::repos::settings::{SettingsRepo, SystemSettings};
use router9::db::{init_memory_pool, migrate};
use router9::routing::RoutingEngine;
use serde_json::{json, Value};
use std::sync::Arc;
use router9::db::repos::usage::{UsageRecord, UsageRepo};
use tower::ServiceExt;

#[tokio::test]
async fn test_api_health_endpoint() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    let req = Request::builder()
        .uri("/api/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["status"], "ok");
    assert_eq!(val["engine"], "native-rust");
    assert_eq!(val["headroom"], "integrated");
}


#[tokio::test]
async fn usage_endpoints_share_persisted_records() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    let mut settings = SettingsRepo::get(&conn).unwrap();
    settings.require_login = false;
    SettingsRepo::save(&conn, &settings).unwrap();
    UsageRepo::record(&conn, &UsageRecord {
        timestamp: chrono::Utc::now().to_rfc3339(),
        id: None,
        provider: Some("fixture".into()),
        model: Some("fixture-model".into()),
        connection_id: None,
        api_key: None,
        endpoint: Some("/v1/chat/completions".into()),
        prompt_tokens: Some(7),
        completion_tokens: Some(3),
        cost: Some(0.25),
        status: Some("ok".into()),
        tokens: None,
        meta: None,
    }).unwrap();
    drop(conn);
    let app = build_router(Arc::new(RoutingEngine::new(pool)));

    for (path, rows_key) in [
        ("/api/usage/history?period=all", "history"),
        ("/api/usage/chart?period=all", "points"),
        ("/api/usage/logs?period=all", "logs"),
    ] {
        let response = app.clone().oneshot(Request::builder().uri(path).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert_eq!(body["period"], "all");
        assert_eq!(body[rows_key].as_array().unwrap().len(), 1, "{path}");
    }
    let response = app.clone().oneshot(Request::builder().uri("/api/usage/request-details?period=all").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let details: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(details["details"].as_array().unwrap().len(), 1);

    let response = app.oneshot(Request::builder().uri("/api/usage/stats?period=all").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let stats: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(stats["totalRequests"], 1);
    assert_eq!(stats["totalPromptTokens"], 7);
    assert_eq!(stats["totalCompletionTokens"], 3);
    assert_eq!(stats["totalCost"], 0.25);
}

#[tokio::test]
async fn provider_connection_can_be_updated_and_keeps_credentials_private() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    let mut settings = SettingsRepo::get(&conn).unwrap();
    settings.require_login = false;
    SettingsRepo::save(&conn, &settings).unwrap();
    drop(conn);
    let app = build_router(Arc::new(RoutingEngine::new(pool)));

    let created = app.clone().oneshot(Request::builder()
        .uri("/api/providers")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(json!({"id":"audit","provider":"test","authType":"apikey","apiKey":"private","isActive":false}).to_string()))
        .unwrap()).await.unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created: Value = serde_json::from_slice(&to_bytes(created.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(!created.to_string().contains("private"));

    let updated = app.clone().oneshot(Request::builder()
        .uri("/api/providers/audit")
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(json!({"isActive":true}).to_string()))
        .unwrap()).await.unwrap();
    assert_eq!(updated.status(), StatusCode::OK);
    let updated: Value = serde_json::from_slice(&to_bytes(updated.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(updated["connection"]["isActive"], true);
    assert!(!updated.to_string().contains("private"));

    let deleted = app.oneshot(Request::builder()
        .uri("/api/providers/audit")
        .method("DELETE")
        .body(Body::empty())
        .unwrap()).await.unwrap();
    assert_eq!(deleted.status(), StatusCode::OK);
}
#[tokio::test]
async fn test_v1_models_endpoint() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    let req = Request::builder()
        .uri("/v1/models")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["object"], "list");
    let data = val["data"].as_array().expect("models array");
    assert!(!data.is_empty());
}

#[tokio::test]
async fn test_dashboard_and_summary_endpoints() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    let mut settings = SettingsRepo::get(&conn).unwrap();
    settings.require_login = false;
    SettingsRepo::save(&conn, &settings).unwrap();
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    let dashboard = app.clone().oneshot(
        Request::builder()
            .uri("/dashboard")
            .method("GET")
            .body(Body::empty())
            .unwrap(),
    ).await.unwrap();
    assert_eq!(dashboard.status(), StatusCode::OK);
    assert_eq!(dashboard.headers()["content-type"], "text/html; charset=utf-8");

    let summary = app.clone().oneshot(
        Request::builder()
            .uri("/api/summary")
            .method("GET")
            .body(Body::empty())
            .unwrap(),
    ).await.unwrap();
    assert_eq!(summary.status(), StatusCode::OK);
    let summary_body = to_bytes(summary.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&summary_body).unwrap();
    assert_eq!(value["providers"]["total"], 0);
    assert_eq!(value["combos"], 0);
    assert_eq!(value["keys"]["active"], 0);

    let nested_dashboard = app.clone().oneshot(
        Request::builder()
            .uri("/dashboard/providers")
            .method("GET")
            .body(Body::empty())
            .unwrap(),
    ).await.unwrap();
    assert_eq!(nested_dashboard.status(), StatusCode::OK);

    let dashboard_css = app.clone().oneshot(
        Request::builder()
            .uri("/dashboard/assets/dashboard.css")
            .method("GET")
            .body(Body::empty())
            .unwrap(),
    ).await.unwrap();
    assert_eq!(dashboard_css.status(), StatusCode::OK);
    assert_eq!(dashboard_css.headers()["content-type"], "text/css; charset=utf-8");

    for path in ["/landing", "/login", "/callback", "/dashboard/settings/pricing"] {
        let response = app.clone().oneshot(
            Request::builder()
                .uri(path)
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        ).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "page route {path}");
    }

    let site_css = app.clone().oneshot(
        Request::builder()
            .uri("/site/assets/site.css")
            .method("GET")
            .body(Body::empty())
            .unwrap(),
    ).await.unwrap();
    assert_eq!(site_css.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_v1_compress_endpoint() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    // Large repetitious JSON array to trigger Headroom SmartCrusher compression
    let mut items = Vec::new();
    for i in 0..40 {
        items.push(json!({
            "id": i,
            "status": "pending",
            "name": format!("User item record with id {}", i),
            "created": "2026-03-29"
        }));
    }
    let large_json_str = serde_json::to_string(&items).unwrap();

    let compress_payload = json!({
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_abc",
            "content": large_json_str
        }]
    });

    let req = Request::builder()
        .uri("/v1/compress")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&compress_payload).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(val.get("messages").is_some());
    let stats = val.get("stats").expect("stats");
    assert!(stats["tokensSaved"].as_u64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn test_api_key_enforcement() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");

    // Enable require_api_key
    let settings = SystemSettings {
        require_api_key: true,
        ..Default::default()
    };
    SettingsRepo::save(&conn, &settings).unwrap();

    // Insert an active key and a deactivated key
    let active_key = ApiKeyItem {
        id: "k1".to_string(),
        key: "sk-valid-key".to_string(),
        name: Some("valid".to_string()),
        machine_id: None,
        is_active: true,
        created_at: "2026-03-29".to_string(),
        access_restricted: false,
        access_allow: None,
    };
    let inactive_key = ApiKeyItem {
        id: "k2".to_string(),
        key: "sk-revoked-key".to_string(),
        name: Some("revoked".to_string()),
        machine_id: None,
        is_active: false,
        created_at: "2026-03-29".to_string(),
        access_restricted: false,
        access_allow: None,
    };
    ApiKeysRepo::insert(&conn, &active_key).unwrap();
    ApiKeysRepo::insert(&conn, &inactive_key).unwrap();
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    // 1. Missing key -> 401
    let req_no_key = Request::builder()
        .uri("/v1/models")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req_no_key).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 2. Revoked key -> 401
    let req_revoked = Request::builder()
        .uri("/v1/models")
        .method("GET")
        .header("Authorization", "Bearer sk-revoked-key")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req_revoked).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 3. Valid key -> 200
    let req_valid = Request::builder()
        .uri("/v1/models")
        .method("GET")
        .header("Authorization", "Bearer sk-valid-key")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req_valid).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
#[tokio::test]
async fn test_slice1_media_and_skills_endpoints() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    // 1. GET /v1/models/image
    let req = Request::builder()
        .uri("/v1/models/image")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["object"], "list");
    assert!(val["data"].as_array().unwrap().iter().any(|m| m["id"] == "openai/dall-e-3"));

    // 2. GET /v1/models/info?id=openai/dall-e-3
    let req = Request::builder()
        .uri("/v1/models/info?id=openai/dall-e-3")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["kind"], "image");

    // 3. POST /v1/embeddings
    let req = Request::builder()
        .uri("/v1/embeddings")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "model": "openai/text-embedding-3-small",
            "input": ["hello world", "rust ai gateway"]
        }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["object"], "list");
    assert_eq!(val["data"].as_array().unwrap().len(), 2);

    // 4. POST /v1/images/generations
    let req = Request::builder()
        .uri("/v1/images/generations")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "model": "openai/dall-e-3",
            "prompt": "sunset over mountains"
        }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(val["data"].as_array().unwrap().len() >= 1);

    // 5. POST /v1/audio/speech
    let req = Request::builder()
        .uri("/v1/audio/speech")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "model": "openai/tts-1",
            "input": "Hello from 9Router"
        }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. GET /v1/audio/voices
    let req = Request::builder()
        .uri("/v1/audio/voices")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 7. POST /v1/videos/generations & GET status
    let req = Request::builder()
        .uri("/v1/videos/generations")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "prompt": "flying eagle over ocean"
        }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    let req_id = val["request_id"].as_str().unwrap();

    let req_poll = Request::builder()
        .uri(format!("/v1/videos/{}", req_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp_poll = app.clone().oneshot(req_poll).await.unwrap();
    assert_eq!(resp_poll.status(), StatusCode::OK);

    // 8. POST /v1/search
    let req = Request::builder()
        .uri("/v1/search")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "model": "tavily",
            "query": "rust ai gateway"
        }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["provider"], "tavily");
    assert!(!val["results"].as_array().unwrap().is_empty());

    // 9. POST /v1/web/fetch
    let req = Request::builder()
        .uri("/v1/web/fetch")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "model": "jina-reader",
            "url": "https://example.com"
        }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["provider"], "jina-reader");
    let text = val["content"]["text"].as_str().unwrap();
    assert!(text.len() > 10, "web/fetch should return non-trivial content");
}

#[tokio::test]
async fn test_slice2_admin_and_proxy_endpoints() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    let mut settings = SettingsRepo::get(&conn).unwrap();
    settings.require_login = false;
    SettingsRepo::save(&conn, &settings).unwrap();
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    // 1. GET /api/version
    let req = Request::builder()
        .uri("/api/version")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["version"], env!("CARGO_PKG_VERSION"));

    // 2. GET /api/quota
    let req = Request::builder()
        .uri("/api/quota")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(val["data"].as_array().unwrap().is_empty());

    // 3. POST /api/proxy-pools and GET /api/proxy-pools
    let req = Request::builder()
        .uri("/api/proxy-pools")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "id": "proxy-test-1",
            "isActive": true,
            "testStatus": "online",
            "data": { "name": "Test Proxy", "url": "http://127.0.0.1:8080" }
        }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .uri("/api/proxy-pools")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val.as_array().unwrap().len(), 1);

    // 4. POST /api/translator/run
    let req = Request::builder()
        .uri("/api/translator/run")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "source_format": "openai",
            "target_format": "claude",
            "payload": {
                "model": "openai/gpt-4o",
                "messages": [{ "role": "user", "content": "hi" }]
            }
        }).to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["target_format"], "claude");
    assert_eq!(val["valid"], true);
}
#[tokio::test]
async fn test_new_admin_endpoints() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    let mut settings = SettingsRepo::get(&conn).unwrap();
    settings.require_login = false;
    SettingsRepo::save(&conn, &settings).unwrap();
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    // 1. GET /api/models
    let req = Request::builder()
        .uri("/api/models")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["object"], "list");
    assert!(val["data"].as_array().unwrap().len() > 0);

    // 2. GET /api/provider-nodes
    let req = Request::builder()
        .uri("/api/provider-nodes")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(val["nodes"].as_array().unwrap().len() > 0);

    // 3. GET /api/models/availability
    let req = Request::builder()
        .uri("/api/models/availability")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["object"], "list");
    assert!(val["data"].as_array().unwrap().len() > 0);

    // 4. GET /api/pxpipe/status
    let req = Request::builder()
        .uri("/api/pxpipe/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["status"], "running");

    // 5. GET /api/pxpipe/health
    let req = Request::builder()
        .uri("/api/pxpipe/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["status"], "healthy");

    // 6. GET /api/headroom/extras
    let req = Request::builder()
        .uri("/api/headroom/extras")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["success"], true);

    // 7. DELETE /api/headroom/extras
    let req = Request::builder()
        .uri("/api/headroom/extras")
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["success"], true);
}
