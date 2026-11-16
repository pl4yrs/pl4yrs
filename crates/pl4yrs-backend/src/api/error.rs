use crate::services::sendspin::SendSpinError;
use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiErrorCode {
    SendspinUnavailable,
    SendspinIoError,
    SendspinBadResponse,
    SendspinConfigError,
    SendspinConfigInvalid,
    SendspinCommandFailure,
    InvalidRequest,
    UnsupportedMediaType,
    UnprocessableEntity,
}

impl ApiErrorCode {
    fn status(self) -> StatusCode {
        match self {
            Self::InvalidRequest => StatusCode::BAD_REQUEST,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::BAD_GATEWAY,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    pub details: Option<String>,
}

impl ApiError {
    pub fn new(code: ApiErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}

impl From<&SendSpinError> for ApiError {
    fn from(e: &SendSpinError) -> Self {
        match e {
            SendSpinError::Connect(io) => ApiError::new(
                ApiErrorCode::SendspinUnavailable,
                "cannot reach sendspin daemon",
            )
            .with_details(&io.to_string()),
            SendSpinError::Io(io) => ApiError::new(
                ApiErrorCode::SendspinIoError,
                "error communicating with sendspin",
            )
            .with_details(&io.to_string()),
            SendSpinError::MalformedResponse => ApiError::new(
                ApiErrorCode::SendspinBadResponse,
                "unexpected response from sendspin",
            ),
            SendSpinError::ConfigAccess(io) => ApiError::new(
                ApiErrorCode::SendspinConfigError,
                "error accessing sendspin config",
            )
            .with_details(&io.to_string()),
            SendSpinError::ConfigInvalid(token) => ApiError::new(
                ApiErrorCode::SendspinConfigInvalid,
                "invalid sendspin configuration",
            )
            .with_details(&token.to_string()),
            SendSpinError::CommandFailed => ApiError::new(
                ApiErrorCode::SendspinCommandFailure,
                "sendspin command failed",
            ),
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        let code = match &rejection {
            JsonRejection::MissingJsonContentType(_) => ApiErrorCode::UnsupportedMediaType,
            JsonRejection::JsonDataError(_) => ApiErrorCode::UnprocessableEntity,
            _ => ApiErrorCode::InvalidRequest,
        };
        ApiError::new(code, "invalid request body").with_details(&rejection.body_text())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.code.status(), Json(self)).into_response()
    }
}
