use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use router9::api::build_router;
use router9::db::repos::keys::{ApiKeyItem, ApiKeysRepo};
use router9::db::repos::settings::{SettingsRepo, SystemSettings};
use router9::db::{init_memory_pool, migrate};
use router9::routing::RoutingEngine;
use serde_json::{json, Value};
use std::sync::Arc;
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
    let dashboard_body = to_bytes(dashboard.into_body(), usize::MAX).await.unwrap();
    let dashboard_html = String::from_utf8(dashboard_body.to_vec()).unwrap();
    assert!(dashboard_html.contains("9Router Dashboard"));
    assert!(dashboard_html.contains("Add provider"));

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
