use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub error: String,
    pub detail: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("internal server error: {0}")]
    Internal(String),
}

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn response(&self) -> ErrorResponse {
        ErrorResponse {
            code: self.status().as_u16(),
            error: self.to_string(),
            detail: match self {
                Self::NotFound(detail)
                | Self::BadRequest(detail)
                | Self::ServiceUnavailable(detail)
                | Self::Internal(detail) => Some(detail.clone()),
                Self::Unauthorized | Self::Forbidden => None,
            },
            request_id: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status(), Json(self.response())).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error.to_string())
    }
}
