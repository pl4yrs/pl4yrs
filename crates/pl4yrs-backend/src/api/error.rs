use crate::services::sendspin::SendSpinError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
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
            SendSpinError::Connect(io) => {
                ApiError::new("SENDSPIN_UNAVAILABLE", "cannot reach sendspin daemon")
                    .with_details(&io.to_string())
            }
            SendSpinError::Io(io) => {
                ApiError::new("SENDSPIN_IO_ERROR", "error communicating with sendspin")
                    .with_details(&io.to_string())
            }
            SendSpinError::MalformedResponse => {
                ApiError::new("SENDSPIN_BAD_RESPONSE", "unexpected response from sendspin")
            }
            SendSpinError::ConfigAccess(io) => {
                ApiError::new("SENDSPIN_CONFIG_ERROR", "error accessing sendspin config")
                    .with_details(&io.to_string())
            }
            SendSpinError::ConfigInvalid(token) => {
                ApiError::new("SENDSPIN_CONFIG_INVALID", "invalid sendspin configuration")
                    .with_details(&token.to_string())
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_GATEWAY, Json(self)).into_response()
    }
}
