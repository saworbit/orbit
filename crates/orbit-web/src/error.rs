//! Error handling for Nebula web interface

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

/// Result type alias for web operations
pub type WebResult<T> = Result<T, WebError>;

/// Web error types
#[derive(Debug, Error)]
pub enum WebError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Authorization error: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            WebError::Auth(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            WebError::Forbidden(_) => (StatusCode::FORBIDDEN, self.to_string()),
            WebError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            WebError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            WebError::Database(_) | WebError::Io(_) | WebError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            WebError::Jwt(_) => (
                StatusCode::UNAUTHORIZED,
                "Invalid or expired token".to_string(),
            ),
            WebError::Json(_) => (StatusCode::BAD_REQUEST, "Invalid JSON".to_string()),
        };

        let body = serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        });

        (status, axum::Json(body)).into_response()
    }
}

impl From<&str> for WebError {
    fn from(msg: &str) -> Self {
        WebError::Internal(msg.to_string())
    }
}

impl From<String> for WebError {
    fn from(msg: String) -> Self {
        WebError::Internal(msg)
    }
}
