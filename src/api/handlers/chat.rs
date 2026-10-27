use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use crate::api::middleware::AppState;
use crate::db::repos::usage::{UsageRecord, UsageRepo};
use crate::routing::create_sse_response;
use crate::translator::ChatRequest;

fn usage_metrics(body: &[u8]) -> (Option<i64>, Option<i64>, Option<f64>) {
    let Ok(body) = serde_json::from_slice::<serde_json::Value>(body) else {
        return (None, None, None);
    };
    let usage = body.get("usage").or_else(|| body.get("usageMetadata")).unwrap_or(&body);
    let token_count = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| usage.get(*key).and_then(serde_json::Value::as_i64))
            .filter(|count| *count >= 0)
    };
    let cost = usage.get("cost").or_else(|| body.get("cost"))
        .and_then(serde_json::Value::as_f64).filter(|cost| cost.is_finite() && *cost >= 0.0);
    (
        token_count(&["prompt_tokens", "input_tokens", "promptTokenCount"]),
        token_count(&["completion_tokens", "output_tokens", "candidatesTokenCount"]),
        cost,
    )
}

pub(crate) fn record_usage(
    pool: crate::db::DbPool,
    provider: String,
    model: String,
    connection_id: Option<String>,
    endpoint: &str,
    body: Option<&[u8]>,
    status: &str,
) {
    let (prompt_tokens, completion_tokens, cost) = body.map(usage_metrics).unwrap_or((None, None, None));
    let record = UsageRecord {
        timestamp: chrono::Utc::now().to_rfc3339(),
        id: None,
        provider: Some(provider),
        model: Some(model),
        connection_id,
        api_key: None,
        endpoint: Some(endpoint.to_string()),
        prompt_tokens,
        completion_tokens,
        cost,
        status: Some(status.to_string()),
        tokens: None,
        meta: None,
    };
    tokio::spawn(async move {
        if let Ok(conn) = pool.get() {
            let _ = UsageRepo::record(&conn, &record);
        }
    });
}

pub async fn handle_chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Response, (StatusCode, String)> {
    let is_stream = req.stream;
    let dispatch = state.engine.dispatch_chat(req).await?;
    let provider = dispatch.provider;
    let model = dispatch.model;
    let connection_id = dispatch.connection_id;
    let resp = dispatch.response;
    let status = resp.status();

    if is_stream {
        if status.is_success() {
            record_usage(state.engine.pool.clone(), provider, model, connection_id, "/v1/chat/completions", None, "accepted");
        }
        let sse = create_sse_response(resp.bytes_stream());
        Ok(sse.into_response())
    } else {
        let body_bytes = resp.bytes().await.map_err(|e| {
            (StatusCode::BAD_GATEWAY, format!("Failed reading upstream response: {}", e))
        })?;

        if status.is_success() {
            record_usage(state.engine.pool.clone(), provider, model, connection_id, "/v1/chat/completions", Some(&body_bytes), "success");
        }

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(body_bytes))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed building response: {}", e)))
    }
}
#[cfg(test)]
mod tests {
    use super::usage_metrics;

    #[test]
    fn reads_openai_anthropic_and_gemini_usage_fields() {
        assert_eq!(usage_metrics(br#"{"usage":{"prompt_tokens":12,"completion_tokens":5,"cost":0.02}}"#), (Some(12), Some(5), Some(0.02)));
        assert_eq!(usage_metrics(br#"{"usage":{"input_tokens":12,"output_tokens":5}}"#), (Some(12), Some(5), None));
        assert_eq!(usage_metrics(br#"{"usageMetadata":{"promptTokenCount":12,"candidatesTokenCount":5}}"#), (Some(12), Some(5), None));
    }

    #[test]
    fn missing_usage_is_not_reported_as_zero() {
        assert_eq!(usage_metrics(br#"{"id":"request-without-usage"}"#), (None, None, None));
        assert_eq!(usage_metrics(b"not json"), (None, None, None));
    }
}
