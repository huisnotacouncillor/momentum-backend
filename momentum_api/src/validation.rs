//! Validation module for API layer
//!
//! This module provides HTTP-specific validation extractors.

pub mod auth;

use axum::{
    Json, async_trait,
    extract::FromRequest,
    http::Request,
};
use axum::body::Body;
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::AppErrorResponse;
use momentum_core::db::models::api::ErrorDetail;
use momentum_core::error::AppError;

/// Validated JSON extractor that validates the payload using validator crate
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S, Body> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppErrorResponse;

    async fn from_request(
        req: Request<Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(value) =
            Json::<T>::from_request(req, state)
                .await
                .map_err(|_| AppErrorResponse(AppError::Validation {
                    message: "Invalid JSON format".to_string(),
                }))?;

        value.validate().map_err(|errors| {
            let error_details: Vec<ErrorDetail> = errors
                .field_errors()
                .iter()
                .flat_map(|(field, field_errors)| {
                    field_errors.iter().map(move |error| ErrorDetail {
                        field: Some(field.to_string()),
                        code: error.code.to_string(),
                        message: error
                            .message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| format!("Validation failed for field: {}", field)),
                    })
                })
                .collect();

            AppErrorResponse(AppError::Validation {
                message: format!("Validation failed with {} errors", error_details.len()),
            })
        })?;

        Ok(ValidatedJson(value))
    }
}
