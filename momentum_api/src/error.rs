//! Error handling for API layer
//!
//! This module provides HTTP-specific error handling.

use axum::{http::StatusCode, Json, response::IntoResponse};
use momentum_core::error::AppError;

/// Wrapper type to implement IntoResponse for AppError
pub struct AppErrorResponse(pub AppError);

impl IntoResponse for AppErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let (status, response) = self.0.to_http_response();
        let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(response)).into_response()
    }
}

/// Convert AppError to an HTTP response
impl From<AppError> for AppErrorResponse {
    fn from(err: AppError) -> Self {
        AppErrorResponse(err)
    }
}
