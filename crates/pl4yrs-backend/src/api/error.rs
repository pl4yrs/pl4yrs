use crate::services::sendspin::SendSpinError;
use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};

use pl4yrs_api::error::{ApiError, ApiErrorCode};

#[derive(Debug)]
pub struct ApiResponseError(ApiError);

impl ApiResponseError {
    pub fn new(code: ApiErrorCode, message: &str) -> Self {
        Self(ApiError::new(code, message))
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.0 = self.0.with_details(details);
        self
    }
}

impl IntoResponse for ApiResponseError {
    fn into_response(self) -> Response {
        let status = match self.0.code {
            ApiErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
            ApiErrorCode::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ApiErrorCode::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::BAD_GATEWAY,
        };

        (status, Json(self.0)).into_response()
    }
}

impl From<&SendSpinError> for ApiResponseError {
    fn from(e: &SendSpinError) -> Self {
        match e {
            SendSpinError::Connect(io) => ApiResponseError::new(
                ApiErrorCode::SendspinUnavailable,
                "cannot reach sendspin daemon",
            )
            .with_details(&io.to_string()),
            SendSpinError::Io(io) => ApiResponseError::new(
                ApiErrorCode::SendspinIoError,
                "error communicating with sendspin",
            )
            .with_details(&io.to_string()),
            SendSpinError::MalformedResponse => ApiResponseError::new(
                ApiErrorCode::SendspinBadResponse,
                "unexpected response from sendspin",
            ),
            SendSpinError::ConfigAccess(io) => ApiResponseError::new(
                ApiErrorCode::SendspinConfigError,
                "error accessing sendspin config",
            )
            .with_details(&io.to_string()),
            SendSpinError::ConfigInvalid(token) => ApiResponseError::new(
                ApiErrorCode::SendspinConfigInvalid,
                "invalid sendspin configuration",
            )
            .with_details(&token.to_string()),
            SendSpinError::CommandFailed => ApiResponseError::new(
                ApiErrorCode::SendspinCommandFailure,
                "sendspin command failed",
            ),
        }
    }
}

impl From<JsonRejection> for ApiResponseError {
    fn from(rejection: JsonRejection) -> Self {
        let code = match &rejection {
            JsonRejection::MissingJsonContentType(_) => ApiErrorCode::UnsupportedMediaType,
            JsonRejection::JsonDataError(_) => ApiErrorCode::UnprocessableEntity,
            _ => ApiErrorCode::InvalidRequest,
        };
        ApiResponseError::new(code, "invalid request body").with_details(&rejection.body_text())
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;
    use crate::services::sendspin::SendSpinError;

    #[test]
    fn sendspin_errors_map_to_codes() {
        use std::io;

        let cases: Vec<(SendSpinError, ApiErrorCode, &str, bool)> = vec![
            (
                SendSpinError::Connect(io::Error::new(io::ErrorKind::NotFound, "nope")),
                ApiErrorCode::SendspinUnavailable,
                "cannot reach sendspin daemon",
                true,
            ),
            (
                SendSpinError::Io(io::Error::new(io::ErrorKind::BrokenPipe, "pipe")),
                ApiErrorCode::SendspinIoError,
                "error communicating with sendspin",
                true,
            ),
            (
                SendSpinError::MalformedResponse,
                ApiErrorCode::SendspinBadResponse,
                "unexpected response from sendspin",
                false,
            ),
            (
                SendSpinError::ConfigAccess(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "denied",
                )),
                ApiErrorCode::SendspinConfigError,
                "error accessing sendspin config",
                true,
            ),
            (
                SendSpinError::ConfigInvalid("port".into()),
                ApiErrorCode::SendspinConfigInvalid,
                "invalid sendspin configuration",
                true,
            ),
            (
                SendSpinError::CommandFailed,
                ApiErrorCode::SendspinCommandFailure,
                "sendspin command failed",
                false,
            ),
        ];

        for (err, expected_code, expected_msg, expect_details) in cases {
            let api_err = ApiResponseError::from(&err).0;
            assert_eq!(api_err.code, expected_code);
            assert_eq!(api_err.message, expected_msg);
            assert_eq!(api_err.details.is_some(), expect_details);
        }
    }

    #[tokio::test]
    async fn into_response_maps_status_and_body() {
        let err = ApiResponseError::new(
            ApiErrorCode::SendspinUnavailable,
            "cannot reach sendspin daemon",
        );
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "SENDSPIN_UNAVAILABLE");
        assert_eq!(json["message"], "cannot reach sendspin daemon");
    }

    #[tokio::test]
    async fn into_response_maps_4xx_codes() {
        for (code, expected) in [
            (ApiErrorCode::InvalidRequest, StatusCode::BAD_REQUEST),
            (
                ApiErrorCode::UnsupportedMediaType,
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ),
            (
                ApiErrorCode::UnprocessableEntity,
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
        ] {
            let response = ApiResponseError::new(code, "ignore me").into_response();
            assert_eq!(response.status(), expected);
        }
    }
}
