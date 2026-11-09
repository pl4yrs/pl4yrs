use crate::api::state;
use crate::player::PlayerCommand;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::convert::Infallible;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::debug;

pub async fn play(State(s): State<state::AppState>) -> impl IntoResponse {
    s.player.send(PlayerCommand::Play);
    debug!(command = "Play", "api → player");
    StatusCode::ACCEPTED
}

pub async fn pause(State(s): State<state::AppState>) -> impl IntoResponse {
    s.player.send(PlayerCommand::Pause);
    debug!(command = "Pause", "api → player");
    StatusCode::ACCEPTED
}

pub async fn resume(State(s): State<state::AppState>) -> impl IntoResponse {
    s.player.send(PlayerCommand::Resume);
    debug!(command = "Resume", "api → player");
    StatusCode::ACCEPTED
}

pub async fn stop(State(s): State<state::AppState>) -> impl IntoResponse {
    s.player.send(PlayerCommand::Stop);
    debug!(command = "Stop", "api → player");
    StatusCode::ACCEPTED
}

pub async fn events(
    State(s): State<state::AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(s.player.subscribe()).filter_map(|msg| match msg {
        Ok(ev) => Some(Ok(
            Event::default().data(serde_json::to_string(&ev).expect("PlayerEvent serializes"))
        )),
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
