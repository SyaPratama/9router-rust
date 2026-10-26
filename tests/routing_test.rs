use router9::db::init_memory_pool;
use router9::db::migrate;
use router9::db::repos::combos::{CombosRepo, ModelCombo};
use router9::db::repos::connections::{ConnectionsRepo, ProviderConnection};
use router9::headroom::HeadroomPipeline;
use router9::routing::account::AccountManager;
use router9::routing::combo::resolve_model_chain;
use router9::routing::token_saver::{TokenSaverPipeline, CAVEMAN_PROMPT_ULTRA, PONYTAIL_PROMPT_ULTRA};
use router9::translator::ChatMessage;
use serde_json::json;

#[test]
fn test_combo_expansion() {
    let pool = init_memory_pool().expect("in-memory DB");
    let conn = pool.get().expect("DB connection");
    migrate(&conn).expect("migration");

    let combo = ModelCombo {
        id: "combo-1".to_string(),
        name: "combo:free-coders".to_string(),
        kind: Some("fallback".to_string()),
        models: vec![
            "deepseek/deepseek-chat".to_string(),
            "openai/gpt-4o-mini".to_string(),
            "anthropic/claude-3-5-haiku".to_string(),
        ],
        created_at: "2026-03-29T00:00:00Z".to_string(),
        updated_at: "2026-03-29T00:00:00Z".to_string(),
    };
    CombosRepo::upsert(&conn, &combo).expect("upsert combo");

    let chain = resolve_model_chain(&conn, "combo:free-coders");
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0], "deepseek/deepseek-chat");
    assert_eq!(chain[1], "openai/gpt-4o-mini");
    assert_eq!(chain[2], "anthropic/claude-3-5-haiku");

    // Single model resolves to itself
    let single = resolve_model_chain(&conn, "gpt-4o");
    assert_eq!(single, vec!["gpt-4o".to_string()]);
}

#[test]
fn test_account_manager_fallback_with_strikes() {
    let pool = init_memory_pool().expect("in-memory DB");
    let conn = pool.get().expect("DB connection");
    migrate(&conn).expect("migration");

    let account1 = ProviderConnection {
        id: "acc_1".to_string(),
        provider: "openai".to_string(),
        auth_type: "api_key".to_string(),
        name: Some("Primary Account".to_string()),
        email: None,
        priority: Some(1),
        is_active: true,
        data: json!({"apiKey": "sk-111"}),
        created_at: "2026-03-29T00:00:00Z".to_string(),
        updated_at: "2026-03-29T00:00:00Z".to_string(),
    };
    let account2 = ProviderConnection {
        id: "acc_2".to_string(),
        provider: "openai".to_string(),
        auth_type: "api_key".to_string(),
        name: Some("Backup Account".to_string()),
        email: None,
        priority: Some(2),
        is_active: true,
        data: json!({"apiKey": "sk-222"}),
        created_at: "2026-03-29T00:00:00Z".to_string(),
        updated_at: "2026-03-29T00:00:00Z".to_string(),
    };

    ConnectionsRepo::upsert(&conn, &account1).unwrap();
    ConnectionsRepo::upsert(&conn, &account2).unwrap();

    let mgr = AccountManager::new();

    // Initially acc_1 comes first because priority is 1
    let initial = mgr.get_available_connections(&conn, "openai");
    assert_eq!(initial[0].id, "acc_1");
    assert_eq!(initial[1].id, "acc_2");

    // Strike account 1 (e.g. on 429)
    mgr.record_strike("acc_1");

    // Now acc_2 must come first because acc_1 has 1 strike
    let fallback = mgr.get_available_connections(&conn, "openai");
    assert_eq!(fallback[0].id, "acc_2");
    assert_eq!(fallback[1].id, "acc_1");

    // Reset strikes
    mgr.clear_strikes("acc_1");
    let restored = mgr.get_available_connections(&conn, "openai");
    assert_eq!(restored[0].id, "acc_1");
}

#[test]
fn test_token_saver_prompt_injections() {
    let mut messages = vec![ChatMessage {
        role: "user".to_string(),
        content: Some(json!("Write code")),
        name: None,
        tool_call_id: None,
        tool_calls: None,
    }];

    let pipeline = HeadroomPipeline::new();
    TokenSaverPipeline::process(
        &mut messages,
        &pipeline,
        true,
        Some("ultra"),
        Some("ultra"),
    );

    // System prompt must have been inserted
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "system");
    let content = messages[0].content.as_ref().unwrap().as_str().unwrap();
    assert!(content.contains(CAVEMAN_PROMPT_ULTRA));
    assert!(content.contains(PONYTAIL_PROMPT_ULTRA));
}
