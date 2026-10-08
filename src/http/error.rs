//! HTTP error mapping (ADR-0024). Classifies a `DatabaseError` as
//! an HTTP status: `NotFound` -> 404, `InvalidOperation` -> 400 (a validation-style
//! failure), everything else -> 500.
//!
//! Exists so handlers can use `?` on a `Result<_, DatabaseError>` and no call site
//! hand-builds an error response.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::error::{ConfigError, DatabaseError};
use crate::media::MediaError;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    InvalidRequest(String),
    /// The request is valid but clashes with work in progress, such as a second scan.
    Conflict(String),
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl From<DatabaseError> for ApiError {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::NotFound(msg) => ApiError::NotFound(msg),
            DatabaseError::InvalidOperation(msg) => ApiError::InvalidRequest(msg),
            other => ApiError::Internal(format!("Database error: {}", other)),
        }
    }
}

impl From<ConfigError> for ApiError {
    /// A setting or path the user gave that cannot be used is a 400; a config file that
    /// cannot be read or written is a 500.
    fn from(err: ConfigError) -> Self {
        match err {
            ConfigError::PathNotFound(_)
            | ConfigError::InvalidDirectory(_)
            | ConfigError::PermissionDenied(_)
            | ConfigError::InvalidPath(_)
            | ConfigError::InvalidValue(_)
            | ConfigError::PortOutOfRange(_) => ApiError::InvalidRequest(err.to_string()),
            other => ApiError::Internal(other.to_string()),
        }
    }
}

impl From<MediaError> for ApiError {
    fn from(err: MediaError) -> Self {
        match err {
            MediaError::FileNotFound(msg) => ApiError::NotFound(msg),
            MediaError::FileTooLarge { .. }
            | MediaError::UnsupportedFormat { .. }
            | MediaError::InvalidMediaType(_)
            | MediaError::InvalidFileId(_) => ApiError::InvalidRequest(err.to_string()),
            other => ApiError::Internal(other.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_of(err: ConfigError) -> StatusCode {
        ApiError::from(err).into_response().status()
    }

    /// `media-mutations-200-on-failure`: a config mutation that fails used to be a 200.
    #[test]
    fn a_setting_that_cannot_be_used_is_a_400_and_an_unwritable_file_a_500() {
        for refused in [
            ConfigError::InvalidValue("log_level".into()),
            ConfigError::InvalidPath("x".into()),
            ConfigError::InvalidDirectory("x".into()),
            ConfigError::PathNotFound("x".into()),
            ConfigError::PermissionDenied("x".into()),
            ConfigError::PortOutOfRange(0),
        ] {
            assert_eq!(status_of(refused), StatusCode::BAD_REQUEST);
        }
        assert_eq!(
            status_of(ConfigError::IoError(std::io::Error::other("disk"))),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            status_of(ConfigError::ConfigFileNotFound),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
