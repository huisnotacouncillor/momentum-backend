use crate::db::models::api::ApiResponse;
use axum::{Json, http::StatusCode, response::IntoResponse};
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

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, response) = match self {
            AppError::Database(ref e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiResponse::<()>::internal_error("Database error"),
                )
            }
            AppError::Pool(ref e) => {
                tracing::error!("Connection pool error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiResponse::<()>::internal_error("Connection error"),
                )
            }
            AppError::Redis(ref e) => {
                tracing::error!("Redis error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiResponse::<()>::internal_error("Cache error"),
                )
            }
            AppError::Auth { ref message } => (
                StatusCode::UNAUTHORIZED,
                ApiResponse::<()>::unauthorized(message),
            ),
            AppError::Validation { ref message } => (
                StatusCode::BAD_REQUEST,
                ApiResponse::<()>::bad_request(message),
            ),
            AppError::NotFound { ref resource } => (
                StatusCode::NOT_FOUND,
                ApiResponse::<()>::not_found(&format!("{} not found", resource)),
            ),
            AppError::Conflict {
                ref message,
                ref field,
                ref code,
            } => (
                StatusCode::CONFLICT,
                ApiResponse::<()>::conflict(message, field.clone(), code.as_deref().unwrap_or("")),
            ),
            AppError::Config(ref e) => {
                tracing::error!("Configuration error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiResponse::<()>::internal_error("Configuration error"),
                )
            }
            AppError::Jwt(ref e) => {
                tracing::error!("JWT error: {}", e);
                (
                    StatusCode::UNAUTHORIZED,
                    ApiResponse::<()>::unauthorized("Invalid token"),
                )
            }
            AppError::Bcrypt(ref e) => {
                tracing::error!("Bcrypt error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiResponse::<()>::internal_error("Password processing error"),
                )
            }
            AppError::Internal(ref message) => {
                tracing::error!("Internal error: {}", message);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiResponse::<()>::internal_error(message),
                )
            }
        };

        (status, Json(response)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

// 便捷的错误创建函数
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
