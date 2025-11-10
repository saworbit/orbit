//! Error types for the web application

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum WebError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<anyhow::Error> for WebError {
    fn from(err: anyhow::Error) -> Self {
        WebError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for WebError {
    fn from(err: serde_json::Error) -> Self {
        WebError::Serialization(err.to_string())
    }
}

#[cfg(feature = "ssr")]
impl From<leptos::ServerFnError> for WebError {
    fn from(err: leptos::ServerFnError) -> Self {
        WebError::Internal(err.to_string())
    }
}

pub type WebResult<T> = Result<T, WebError>;
