use crate::api::handlers;
use crate::api::state;
use axum::{Router, routing::post};
use tower_http::trace::TraceLayer;

pub fn get_router(state: state::AppState) -> Router {
    Router::new()
        .route("/play", post(handlers::play))
        .route("/pause", post(handlers::pause))
        .route("/resume", post(handlers::resume))
        .route("/stop", post(handlers::stop))
        .route("/events", post(handlers::events))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
