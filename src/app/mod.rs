pub mod admin;
pub mod auth;
pub mod dashboard;
pub mod v1;
pub mod pages;

use axum::{middleware as axum_mw, Router};
use std::sync::Arc;

use crate::api::middleware::{security_middleware, AppState};
use crate::routing::RoutingEngine;

pub fn build_router(engine: Arc<RoutingEngine>) -> Router {
    let state = AppState { engine };

    Router::new()
        .merge(v1::routes())
        .merge(admin::routes())
        .merge(auth::routes())
        .merge(dashboard::routes())
        .merge(pages::routes())
        .route("/", axum::routing::get(crate::web::serve_dashboard))
        .layer(axum_mw::from_fn_with_state(state.clone(), security_middleware))
        .with_state(state)
}
