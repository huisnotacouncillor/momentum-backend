//! Validation module - pure Rust, no HTTP dependencies
//!
//! This module contains validation rules and error types that don't depend
//! on any web frameworks. HTTP-specific extractors like `ValidatedJson`
//! live in momentum_api.

pub mod auth;
pub mod comment;
pub mod cycle;
pub mod invitation;
pub mod issue;
pub mod label;
pub mod project;
pub mod project_status;
pub mod workflow;
pub mod workspace;
pub mod workspace_member;

use serde::Serialize;

/// Validation error detail
#[derive(Debug, Clone, Serialize)]
pub struct ValidationErrorDetail {
    pub field: Option<String>,
    pub code: String,
    pub message: String,
}

/// Validation error response helper
#[derive(Debug, Clone, Serialize)]
pub struct ValidationErrorResponse {
    pub errors: Vec<ValidationErrorDetail>,
}

impl ValidationErrorResponse {
    pub fn from_validation_errors(errors: validator::ValidationErrors) -> Self {
        let error_details: Vec<ValidationErrorDetail> = errors
            .field_errors()
            .iter()
            .flat_map(|(field, field_errors)| {
                field_errors.iter().map(move |error| ValidationErrorDetail {
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

        Self {
            errors: error_details,
        }
    }
}

/// Common validation rules
pub mod rules {
    use validator::ValidationError;

    /// Validate password strength
    pub fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
        let mut score = 0;

        // Length check
        if password.len() >= 8 {
            score += 1;
        }

        // Contains lowercase
        if password.chars().any(|c| c.is_lowercase()) {
            score += 1;
        }

        // Contains uppercase
        if password.chars().any(|c| c.is_uppercase()) {
            score += 1;
        }

        // Contains number
        if password.chars().any(|c| c.is_numeric()) {
            score += 1;
        }

        // Contains special character
        if password
            .chars()
            .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c))
        {
            score += 1;
        }

        if score < 3 {
            return Err(ValidationError::new("weak_password"));
        }

        Ok(())
    }

    /// Validate username format
    pub fn validate_username_format(username: &str) -> Result<(), ValidationError> {
        // Only allow alphanumeric, underscore, and hyphen
        if !username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ValidationError::new("invalid_username_format"));
        }

        // Cannot start with number
        if username.chars().next().is_some_and(|c| c.is_numeric()) {
            return Err(ValidationError::new("username_starts_with_number"));
        }

        Ok(())
    }

    /// Validate workspace URL key format
    pub fn validate_workspace_url_key(url_key: &str) -> Result<(), ValidationError> {
        // Only allow lowercase letters, numbers, and hyphens
        if !url_key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_numeric() || c == '-')
        {
            return Err(ValidationError::new("invalid_url_key_format"));
        }

        // Cannot start or end with hyphen
        if url_key.starts_with('-') || url_key.ends_with('-') {
            return Err(ValidationError::new("url_key_invalid_hyphens"));
        }

        Ok(())
    }
}
