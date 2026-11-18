use crate::api::handlers;
use crate::api::state;
use axum::routing::post;
use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

pub fn get_router(state: state::AppState) -> Router {
    Router::new()
        .route("/status", get(handlers::status))
        .route("/play", post(handlers::play))
        .route("/pause", post(handlers::pause))
        .route("/status/events", get(handlers::status_events))
        .route("/config", get(handlers::get_config))
        .route("/volume", post(handlers::set_volume))
        .route("/health", get(handlers::health))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{default_handler, mock_sendspin};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use pl4yrs_api::{
        error::{ApiError, ApiErrorCode},
        playback::{PlayerState, PlayerStatus},
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    fn app_state(dir: &tempfile::TempDir) -> state::AppState {
        state::AppState::new(dir.path().join("fake.sock"), dir.path().join("fake.conf"))
    }

    #[tokio::test]
    async fn volume_rejects_invalid_requests() {
        let dir = tempfile::tempdir().unwrap();

        let cases = [
            (
                None,
                r#"{"volume":42}"#,
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                ApiErrorCode::UnsupportedMediaType,
            ),
            (
                Some("application/json"),
                "{invalid json",
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidRequest,
            ),
            (
                Some("application/json"),
                r#"{"volume":142}"#,
                StatusCode::UNPROCESSABLE_ENTITY,
                ApiErrorCode::UnprocessableEntity,
            ),
            (
                Some("application/json"),
                r#"{}"#,
                StatusCode::UNPROCESSABLE_ENTITY,
                ApiErrorCode::UnprocessableEntity,
            ),
        ];

        for (content_type, body, expected_status, expected_code) in cases {
            let mut request = Request::post("/volume");
            if let Some(content_type) = content_type {
                request = request.header("content-type", content_type);
            }
            let response = get_router(app_state(&dir))
                .oneshot(request.body(Body::from(body)).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), expected_status, "body: {body:?}");
            let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
            let error: ApiError = serde_json::from_slice(&bytes).unwrap();

            assert_eq!(error.code, expected_code, "body: {body:?}");
            assert!(error.details.as_deref().is_some_and(|s| !s.is_empty()));
        }
    }

    #[tokio::test]
    async fn routes_reject_unknown_paths_and_wrong_methods() {
        let dir = tempfile::tempdir().unwrap();

        let cases = [
            ("GET", "/missing", StatusCode::NOT_FOUND),
            ("GET", "/play", StatusCode::METHOD_NOT_ALLOWED),
            ("GET", "/pause", StatusCode::METHOD_NOT_ALLOWED),
            ("GET", "/volume", StatusCode::METHOD_NOT_ALLOWED),
            ("POST", "/status", StatusCode::METHOD_NOT_ALLOWED),
        ];

        for (method, path, error) in cases {
            let request = Request::builder()
                .method(method)
                .uri(path)
                .body(Body::empty())
                .unwrap();
            let response = get_router(app_state(&dir)).oneshot(request).await.unwrap();

            assert_eq!(response.status(), error, "{method} {path}");
        }
    }

    #[tokio::test]
    async fn action_routes_send_expected_commands() {
        let cases = [
            ("/play", "", "play"),
            ("/pause", "", "pause"),
            ("/volume", r#"{"volume":42}"#, "vol 42"),
        ];

        for (path, body, expected_command) in cases {
            let dir = tempfile::tempdir().unwrap();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            let socket = mock_sendspin(&dir, move |command| {
                tx.send(command.to_owned()).unwrap();
                default_handler(command)
            });

            let state = state::AppState::new(socket, dir.path().join("unused.conf"));
            let app = get_router(state);

            let request = Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::NO_CONTENT, "{path}");
            assert_eq!(rx.try_recv().unwrap(), expected_command, "{path}");
            assert_eq!(rx.try_recv().unwrap(), "status", "{path}");
        }
    }

    #[tokio::test]
    async fn health_route_returns_ok() {
        let dir = tempfile::tempdir().unwrap();
        let app = get_router(app_state(&dir));

        let response = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(body, json!({ "status": "OK" }));
    }

    #[tokio::test]
    async fn status_route_returns_player_status() {
        let dir = tempfile::tempdir().unwrap();
        let app = get_router(app_state(&dir));

        let response = app
            .oneshot(Request::get("/status").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let status: PlayerStatus = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(status.state, PlayerState::Unknown);
    }

    #[tokio::test]
    async fn config_route_returns_parsed_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("fake.conf"),
            "name = testplayer\noutput = sysdefault\nbuffer-ms = 200\n",
        )
        .unwrap();

        let app = get_router(app_state(&dir));

        let response = app
            .oneshot(Request::get("/config").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let config: Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(config["name"], "testplayer");
        assert_eq!(config["output"], "sysdefault");
        assert_eq!(config["buffer_ms"], 200);
    }
}
