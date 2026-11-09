use crate::api::handlers;
use crate::api::state;
use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

pub fn get_router(state: state::AppState) -> Router {
    Router::new()
        .route("/status", get(handlers::status))
        .route("/status/events", get(handlers::status_events))
        .route("/config", get(handlers::get_config))
        .route("/health", get(handlers::health))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
