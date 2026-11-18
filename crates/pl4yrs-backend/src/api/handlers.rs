use crate::api::error::ApiResponseError;
use crate::api::state;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{extract::State, response::Json};
use futures_core::Stream;
use pl4yrs_api::control::SetVolume;
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
) -> Result<Json<crate::services::sendspin::SendSpinConfig>, ApiResponseError> {
    s.sendspin_service
        .get_config()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::warn!(error = %e, "sendspin get config failed");
            ApiResponseError::from(&e)
        })
}

pub async fn set_volume(
    State(s): State<state::AppState>,
    body: Result<Json<SetVolume>, JsonRejection>,
) -> Result<StatusCode, ApiResponseError> {
    let Json(body) = body.map_err(ApiResponseError::from)?;
    s.sendspin_service
        .set_volume(body.volume)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            tracing::warn!(error = %e, "sendspin volume failed");
            ApiResponseError::from(&e)
        })
}

pub async fn play(State(s): State<state::AppState>) -> Result<StatusCode, ApiResponseError> {
    s.sendspin_service
        .play()
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            tracing::warn!(error = %e, "sendspin play failed");
            ApiResponseError::from(&e)
        })
}

pub async fn pause(State(s): State<state::AppState>) -> Result<StatusCode, ApiResponseError> {
    s.sendspin_service
        .pause()
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            tracing::warn!(error = %e, "sendspin pause failed");
            ApiResponseError::from(&e)
        })
}

pub async fn health(State(_s): State<state::AppState>) -> Json<Value> {
    debug!(command = "Health", "internal");
    Json(json!({ "status": "OK" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{default_handler, mock_sendspin};
    use axum::{body::to_bytes, response::IntoResponse};
    use pl4yrs_api::{
        control::Volume,
        error::{ApiError, ApiErrorCode},
        playback::PlayerState,
    };
    use std::path::PathBuf;
    use tokio::sync::mpsc::UnboundedReceiver;

    async fn assert_api_error(error: ApiResponseError, expected_code: ApiErrorCode) -> ApiError {
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let error: ApiError = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(error.code, expected_code);
        error
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app_state =
            state::AppState::new(PathBuf::from("/fake.sock"), PathBuf::from("/fake.conf"));

        let Json(body) = health(State(app_state)).await;

        assert_eq!(body, json!({ "status": "OK" }));
    }

    #[tokio::test]
    async fn status_returns_current_watch_value() {
        let state = state::AppState::new(PathBuf::from("/fake.sock"), PathBuf::from("/fake.conf"));
        let Json(status) = status(State(state)).await;

        assert!(matches!(status.state, PlayerState::Unknown));
    }

    #[tokio::test]
    async fn status_reflects_command_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = mock_sendspin(&dir, default_handler);
        let app_state = state::AppState::new(socket_path, PathBuf::from("/fake.conf"));

        let result = play(State(app_state.clone())).await.unwrap();
        assert_eq!(result, StatusCode::NO_CONTENT);

        let Json(player_status) = status(State(app_state)).await;

        assert_eq!(player_status.state, PlayerState::Playing);
        assert_eq!(player_status.media.unwrap().title, "No Stairway to Heaven");
    }

    #[tokio::test]
    async fn get_config_returns_parsed_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("sendspin-cli.conf");
        std::fs::write(
            &config_path,
            "name = testplayer\noutput = sysdefault\nbuffer-ms = 200\n",
        )
        .unwrap();

        let app_state = state::AppState::new(dir.path().join("fake.sock"), config_path);
        let Json(config) = get_config(State(app_state)).await.unwrap();

        assert_eq!(config.name, "testplayer");
        assert_eq!(config.output, "sysdefault");
        assert_eq!(config.buffer_ms, 200);
    }

    #[tokio::test]
    async fn get_config_missing_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let app_state =
            state::AppState::new(dir.path().join("fake.sock"), dir.path().join("fake.conf"));

        let error = get_config(State(app_state)).await.unwrap_err();
        let error = assert_api_error(error, ApiErrorCode::SendspinConfigError).await;
        assert!(error.details.is_some());
    }

    #[tokio::test]
    async fn get_config_unknown_key_returns_config_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("sendspin-cli.conf");
        std::fs::write(
            &config_path,
            "name = testplayer\noutput = sysdefault\ngarbage = 314\n",
        )
        .unwrap();

        let app_state = state::AppState::new(dir.path().join("fake.sock"), config_path);
        let error = get_config(State(app_state)).await.unwrap_err();
        let error = assert_api_error(error, ApiErrorCode::SendspinConfigInvalid).await;
        assert_eq!(error.details.as_deref(), Some("garbage = 314"));
    }

    fn setup_channel_and_state() -> (
        tempfile::TempDir,
        UnboundedReceiver<String>,
        state::AppState,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let socket = mock_sendspin(&dir, move |command| {
            tx.send(command.to_owned()).unwrap();
            default_handler(command)
        });
        let app_state = state::AppState::new(socket, dir.path().join("/fake.conf"));

        (dir, rx, app_state)
    }

    async fn call_action(
        state: state::AppState,
        action: &str,
    ) -> Result<StatusCode, ApiResponseError> {
        match action {
            "play" => play(State(state)).await,
            "pause" => pause(State(state)).await,
            "volume" => {
                set_volume(
                    State(state),
                    Ok(Json(SetVolume {
                        volume: Volume::new(42).unwrap(),
                    })),
                )
                .await
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn pause_sends_command_and_refreshes_status() {
        let (_dir, mut rx, app_state) = setup_channel_and_state();
        let result = call_action(app_state, "pause").await.unwrap();

        assert_eq!(result, StatusCode::NO_CONTENT);
        assert_eq!(rx.try_recv().unwrap(), "pause");
        assert_eq!(rx.try_recv().unwrap(), "status");
    }

    #[tokio::test]
    async fn play_sends_command_and_refreshes_status() {
        let (_dir, mut rx, app_state) = setup_channel_and_state();
        let result = call_action(app_state, "play").await.unwrap();

        assert_eq!(result, StatusCode::NO_CONTENT);
        assert_eq!(rx.try_recv().unwrap(), "play");
        assert_eq!(rx.try_recv().unwrap(), "status");
    }

    #[tokio::test]
    async fn set_volume_sends_command_and_refreshes_status() {
        let (_dir, mut rx, app_state) = setup_channel_and_state();
        let result = call_action(app_state, "volume").await.unwrap();

        assert_eq!(result, StatusCode::NO_CONTENT);
        assert_eq!(rx.try_recv().unwrap(), "vol 42");
        assert_eq!(rx.try_recv().unwrap(), "status");
    }

    #[tokio::test]
    async fn actions_report_unavailable_daemon() {
        let dir = tempfile::tempdir().unwrap();

        for action in ["play", "pause", "volume"] {
            let app_state = state::AppState::new(
                dir.path().join("missing.sock"),
                dir.path().join("fake.conf"),
            );

            let error = call_action(app_state, action).await.unwrap_err();

            assert_api_error(error, ApiErrorCode::SendspinUnavailable).await;
        }
    }

    #[tokio::test]
    async fn actions_report_failed_and_malformed_responses() {
        for (reply, expected_code) in [
            ("error", ApiErrorCode::SendspinCommandFailure),
            ("", ApiErrorCode::SendspinBadResponse),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let socket = mock_sendspin(&dir, move |_| reply.to_owned());
            let app_state = state::AppState::new(socket, dir.path().join("fake.conf"));

            for action in ["play", "pause", "volume"] {
                let error = call_action(app_state.clone(), action).await.unwrap_err();
                assert_api_error(error, expected_code).await;
            }
        }
    }
}
