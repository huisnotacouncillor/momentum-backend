//! Core error types - pure Rust, no HTTP dependencies
//!
//! This module contains core error types that don't depend on any web frameworks.
//! The `IntoResponse` implementation for HTTP APIs is in momentum_api.

use crate::db::models::api::ApiResponse;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("Pool error: {0}")]
    Pool(#[from] r2d2::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Authentication error: {message}")]
    Auth { message: String },

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Conflict: {message}")]
    Conflict {
        message: String,
        field: Option<String>,
        code: Option<String>,
    },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Bcrypt error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),

    #[error("Internal server error: {0}")]
    Internal(String),
}

pub type AppErrorResponse = (u16, ApiResponse<()>);

/// Convert AppError to HTTP status code and response
impl AppError {
    pub fn to_http_response(&self) -> AppErrorResponse {
        use crate::db::models::api::ApiResponse;

        match self {
            AppError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (
                    500,
                    ApiResponse::<()>::internal_error("Database error"),
                )
            }
            AppError::Pool(e) => {
                tracing::error!("Connection pool error: {}", e);
                (
                    500,
                    ApiResponse::<()>::internal_error("Connection error"),
                )
            }
            AppError::Redis(e) => {
                tracing::error!("Redis error: {}", e);
                (
                    500,
                    ApiResponse::<()>::internal_error("Cache error"),
                )
            }
            AppError::Auth { message } => (
                401,
                ApiResponse::<()>::unauthorized(message),
            ),
            AppError::Validation { message } => (
                400,
                ApiResponse::<()>::bad_request(message),
            ),
            AppError::NotFound { resource } => (
                404,
                ApiResponse::<()>::not_found(&format!("{} not found", resource)),
            ),
            AppError::Conflict {
                message,
                field,
                code,
            } => (
                409,
                ApiResponse::<()>::conflict(message, field.clone(), code.as_deref().unwrap_or("")),
            ),
            AppError::Config(e) => {
                tracing::error!("Configuration error: {}", e);
                (
                    500,
                    ApiResponse::<()>::internal_error("Configuration error"),
                )
            }
            AppError::Jwt(e) => {
                tracing::error!("JWT error: {}", e);
                (
                    401,
                    ApiResponse::<()>::unauthorized("Invalid token"),
                )
            }
            AppError::Bcrypt(e) => {
                tracing::error!("Bcrypt error: {}", e);
                (
                    500,
                    ApiResponse::<()>::internal_error("Password processing error"),
                )
            }
            AppError::Internal(message) => {
                tracing::error!("Internal error: {}", message);
                (
                    500,
                    ApiResponse::<()>::internal_error(message),
                )
            }
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

// Convenience error creation functions
impl AppError {
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    pub fn conflict_with_code(
        message: impl Into<String>,
        field: Option<String>,
        code: impl Into<String>,
    ) -> Self {
        Self::Conflict {
            message: message.into(),
            field,
            code: Some(code.into()),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }
}
