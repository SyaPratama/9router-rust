use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Instant;
use uuid::Uuid;
use crate::api::middleware::AppState;

#[derive(Debug, Clone)]
struct VideoJob {
    id: String,
    created_at: Instant,
    model: String,
    duration: u32,
}

static VIDEO_JOBS: LazyLock<Arc<RwLock<HashMap<String, VideoJob>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

#[derive(Debug, Deserialize)]
pub struct VideoGenerationRequest {
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub duration: Option<u32>,
    pub aspect_ratio: Option<String>,
    pub resolution: Option<String>,
    pub image: Option<serde_json::Value>,
    pub video: Option<serde_json::Value>,
}

fn create_video_job(payload: VideoGenerationRequest) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    let req_id = Uuid::new_v4().to_string();
    let conn_id = format!("conn_{}", &req_id[..8]);
    let model = payload.model.unwrap_or_else(|| "grok-imagine-video".to_string());
    let duration = payload.duration.unwrap_or(8);

    let job = VideoJob {
        id: req_id.clone(),
        created_at: Instant::now(),
        model,
        duration,
    };

    if let Ok(mut map) = VIDEO_JOBS.write() {
        map.insert(req_id.clone(), job);
    }

    let mut headers = HeaderMap::new();
    if let Ok(hval) = HeaderValue::from_str(&conn_id) {
        headers.insert(HeaderName::from_static("x-9router-connection-id"), hval);
    }

    (
        StatusCode::OK,
        headers,
        Json(json!({ "request_id": req_id })),
    )
}

pub async fn handle_video_generations(
    State(_state): State<AppState>,
    Json(payload): Json<VideoGenerationRequest>,
) -> Result<Response, (StatusCode, String)> {
    let (status, headers, body) = create_video_job(payload);
    let mut resp = body.into_response();
    *resp.status_mut() = status;
    resp.headers_mut().extend(headers);
    Ok(resp)
}

pub async fn handle_video_edits(
    State(_state): State<AppState>,
    Json(payload): Json<VideoGenerationRequest>,
) -> Result<Response, (StatusCode, String)> {
    let (status, headers, body) = create_video_job(payload);
    let mut resp = body.into_response();
    *resp.status_mut() = status;
    resp.headers_mut().extend(headers);
    Ok(resp)
}

pub async fn handle_video_extensions(
    State(_state): State<AppState>,
    Json(payload): Json<VideoGenerationRequest>,
) -> Result<Response, (StatusCode, String)> {
    let (status, headers, body) = create_video_job(payload);
    let mut resp = body.into_response();
    *resp.status_mut() = status;
    resp.headers_mut().extend(headers);
    Ok(resp)
}

pub async fn get_video_status(
    State(_state): State<AppState>,
    Path(request_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let job = {
        let map = VIDEO_JOBS.read().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock error: {}", e))
        })?;
        map.get(&request_id).cloned()
    };

    match job {
        Some(j) => {
            let elapsed = j.created_at.elapsed().as_secs();
            if elapsed < 1 {
                Ok(Json(json!({
                    "status": "pending",
                    "progress": 45
                })))
            } else {
                Ok(Json(json!({
                    "status": "done",
                    "video": {
                        "url": format!("https://cdn.9router.internal/videos/{}.mp4", j.id),
                        "duration": j.duration
                    },
                    "model": j.model
                })))
            }
        }
        None => {
            // Not found in local memory, return simulated completed status or 404
            Ok(Json(json!({
                "status": "done",
                "video": {
                    "url": format!("https://cdn.9router.internal/videos/{}.mp4", request_id),
                    "duration": 8
                },
                "model": "grok-imagine-video"
            })))
        }
    }
}
