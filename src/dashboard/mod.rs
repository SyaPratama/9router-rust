use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/dashboard/assets/"]
struct Asset;

pub async fn serve_dashboard() -> impl IntoResponse {
    match Asset::get("index.html") {
        Some(content) => Html(content.data.to_vec()).into_response(),
        None => (StatusCode::NOT_FOUND, "Dashboard asset not found").into_response(),
    }
}
