use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use crate::api::middleware::AppState;
use crate::db::repos::combos::CombosRepo;
use crate::db::repos::connections::ConnectionsRepo;

pub async fn list_models(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.engine.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB connection error: {}", e))
    })?;

    let mut model_list = Vec::new();

    // Default base models
    let default_models = [
        "gpt-4o", "gpt-4o-mini", "claude-3-5-sonnet", "claude-3-5-haiku",
        "deepseek-chat", "deepseek-reasoner", "gemini-1.5-pro", "gemini-1.5-flash",
    ];
    for m in default_models {
        model_list.push(json!({
            "id": m,
            "object": "model",
            "created": 1700000000,
            "owned_by": "system"
        }));
    }

    // Active provider connections
    if let Ok(conns) = ConnectionsRepo::list_all(&conn) {
        for c in conns {
            if c.is_active {
                model_list.push(json!({
                    "id": format!("{}/auto", c.provider),
                    "object": "model",
                    "created": 1700000000,
                    "owned_by": c.provider
                }));
            }
        }
    }

    // Combos
    if let Ok(combos) = CombosRepo::list_all(&conn) {
        for combo in combos {
            model_list.push(json!({
                "id": combo.name,
                "object": "model",
                "created": 1700000000,
                "owned_by": "combo"
            }));
        }
    }

    Ok(Json(json!({
        "object": "list",
        "data": model_list
    })))
}

pub async fn get_model(
    State(_state): State<AppState>,
    Path(model): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json(json!({
        "id": model,
        "object": "model",
        "created": 1700000000,
        "owned_by": "9router"
    })))
}
