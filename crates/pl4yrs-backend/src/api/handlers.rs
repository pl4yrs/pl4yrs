use crate::api::error::ApiError;
use crate::api::state;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{extract::State, response::Json};
use futures_core::Stream;
use pl4yrs_api::playback::PlayerStatus;
use serde_json::{Value, json};
use std::convert::Infallible;
use std::time::Duration;
use tracing::debug;

pub async fn status(State(s): State<state::AppState>) -> Json<PlayerStatus> {
    let status = s.sendspin_service.status();
    Json(PlayerStatus::from(&status))
}

pub async fn status_events(
    State(s): State<state::AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>> + Send + 'static> {
    let mut rx = s.sendspin_service.status_rx;

    let stream = async_stream::stream! {
        let status = rx.borrow_and_update().clone();
        let response = PlayerStatus::from(&status);
        yield Ok(Event::default().json_data(response).unwrap());

        loop {
            if rx.changed().await.is_err() { break; }
            let status = rx.borrow_and_update().clone();
            let response = PlayerStatus::from(&status);
            yield Ok(Event::default().json_data(response).unwrap());
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
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
