use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    pub details: Option<String>,
}

impl ApiError {
    pub fn new(code: ApiErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.to_owned(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_owned());
        self
    }
}
