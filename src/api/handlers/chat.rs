use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use crate::api::middleware::AppState;
use crate::db::repos::usage::{UsageRecord, UsageRepo};
use crate::routing::create_sse_response;
use crate::translator::ChatRequest;

pub async fn handle_chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Response, (StatusCode, String)> {
    let is_stream = req.stream;
    let model_name = req.model.clone();

    let resp = state.engine.dispatch_chat(req).await?;

    if is_stream {
        let stream = resp.bytes_stream();
        let sse = create_sse_response(stream);
        Ok(sse.into_response())
    } else {
        let status = resp.status();
        let body_bytes = resp.bytes().await.map_err(|e| {
            (StatusCode::BAD_GATEWAY, format!("Failed reading upstream response: {}", e))
        })?;

        // Asynchronously record usage if successful
        if status.is_success() {
            let pool = state.engine.pool.clone();
            let model = model_name.clone();
            tokio::spawn(async move {
                if let Ok(conn) = pool.get() {
                    let record = UsageRecord {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        provider: None,
                        model: Some(model),
                        connection_id: None,
                        api_key: None,
                        endpoint: Some("/v1/chat/completions".to_string()),
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        cost: 0.0,
                        status: Some("success".to_string()),
                        tokens: None,
                        meta: None,
                    };
                    let _ = UsageRepo::record(&conn, &record);
                }
            });
        }

        let resp_builder = Response::builder()
            .status(status)
            .header("Content-Type", "application/json");

        resp_builder.body(axum::body::Body::from(body_bytes)).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed building response: {}", e))
        })
    }
}
