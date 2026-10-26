pub mod middleware;
pub mod handlers;

use axum::Router;
use std::sync::Arc;
use crate::routing::RoutingEngine;

pub fn build_router(engine: Arc<RoutingEngine>) -> Router {
    crate::app::build_router(engine)
}
