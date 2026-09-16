//! HTTP error mapping (ADR-0024). Mirrors `src/grpc/error.rs`'s
//! `From<DatabaseError> for tonic::Status`, but to HTTP status codes instead of
//! gRPC codes, with the same classification: `NotFound` -> 404,
//! `InvalidOperation` -> 400 (a validation-style failure), everything else -> 500.
//!
//! Exists so handlers can use `?` on a `Result<_, DatabaseError>` and no call site
//! hand-builds an error response.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::error::DatabaseError;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    InvalidRequest(String),
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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}
