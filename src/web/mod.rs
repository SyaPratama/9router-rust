use axum::extract::Path;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/web/assets/"]
struct Asset;

pub async fn serve_dashboard() -> impl IntoResponse {
    serve_named_page("index.html")
}

pub async fn serve_page(uri: axum::extract::OriginalUri) -> impl IntoResponse {
    let path = uri.0.path();
    let page = match path {
        "/landing" => "landing.html",
        "/callback" => "callback.html",
        "/dashboard/settings/pricing" => "pricing.html",
        _ => "index.html",
    };

    if Asset::get(page).is_some() {
        serve_named_page(page)
    } else {
        serve_named_page("index.html")
    }
}

fn serve_named_page(page: &str) -> Response {
    match Asset::get(page) {
        Some(content) => (
            [
                (header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8")),
                (header::CACHE_CONTROL, HeaderValue::from_static("no-cache, no-store, must-revalidate")),
            ],
            content.data.to_vec(),
        ).into_response(),
        None => (StatusCode::NOT_FOUND, "Dashboard asset not found").into_response(),
    }
}

pub async fn serve_asset(Path(path): Path<String>) -> impl IntoResponse {
    let clean_path = path.trim_start_matches('/');
    let asset = Asset::get(clean_path)
        .or_else(|| {
            if let Some(stripped) = clean_path.strip_prefix("assets/") {
                Asset::get(stripped)
            } else {
                None
            }
        })
        .or_else(|| {
            if !clean_path.starts_with("providers/") {
                Asset::get(&format!("providers/{}", clean_path))
            } else {
                None
            }
        });

    let asset = match asset {
        Some(a) => a,
        None => return (StatusCode::NOT_FOUND, "Dashboard asset not found").into_response(),
    };

    let content_type = match clean_path.rsplit('.').next().unwrap_or("").to_lowercase().as_str() {
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "json" => "application/json",
        "html" => "text/html; charset=utf-8",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        _ => "application/octet-stream",
    };

    (
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache, no-store, must-revalidate")),
        ],
        asset.data.to_vec(),
    )
        .into_response()
}
