use crate::api::error::ApiError;
use crate::api::state;
use axum::{extract::State, response::Json};
use serde_json::{Value, json};
use tracing::debug;

pub async fn status(
    State(s): State<state::AppState>,
) -> Result<Json<crate::services::sendspin::SendSpinStatus>, ApiError> {
    s.sendspin_service
        .get_status()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::warn!(error = %e, "sendspin status failed");
            ApiError::from(&e)
        })
}

pub async fn get_config(
    State(s): State<state::AppState>,
) -> Result<Json<crate::services::sendspin::SendSpinConfig>, ApiError> {
    s.sendspin_service
        .get_config()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::warn!(error = %e, "sendspin status failed");
            ApiError::from(&e)
        })
}

pub async fn health(State(_s): State<state::AppState>) -> Json<Value> {
    debug!(command = "Health", "internal");
    Json(json!({ "status": "OK" }))
}
