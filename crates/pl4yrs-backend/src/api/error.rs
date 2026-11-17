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
