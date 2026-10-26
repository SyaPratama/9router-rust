use axum::extract::Path;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/dashboard/assets/"]
struct Asset;

pub async fn serve_dashboard() -> impl IntoResponse {
    serve_named_page("index.html")
}

pub async fn serve_page(uri: axum::extract::OriginalUri) -> impl IntoResponse {
    let page = match uri.0.path() {
        "/landing" => "landing.html",
        "/login" => "login.html",
        "/callback" => "callback.html",
        "/dashboard/settings/pricing" => "pricing.html",
        _ => "dashboard.html",
    };
    serve_named_page(page)
}

fn serve_named_page(page: &str) -> axum::response::Response {
    match Asset::get(page) {
        Some(content) => Html(content.data.to_vec()).into_response(),
        None => (StatusCode::NOT_FOUND, "Dashboard asset not found").into_response(),
    }
}

pub async fn serve_asset(Path(path): Path<String>) -> impl IntoResponse {
    let asset = match Asset::get(path.as_str()) {
        Some(asset) => asset,
        None => return (StatusCode::NOT_FOUND, "Dashboard asset not found").into_response(),
    };

    let content_type = match path.rsplit('.').next() {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    };

    (
        [(header::CONTENT_TYPE, HeaderValue::from_static(content_type))],
        asset.data.to_vec(),
    )
        .into_response()
}
