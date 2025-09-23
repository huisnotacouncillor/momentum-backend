use crate::error::AppError;

pub fn validate_create_workspace(name: &str, url_key: &str) -> Result<(), AppError> {
    if name.trim().is_empty() { return Err(AppError::validation("Workspace name is required")); }
    if url_key.trim().is_empty() { return Err(AppError::validation("Workspace url_key is required")); }
    if !url_key.chars().all(|c| c.is_ascii_lowercase() || c.is_numeric() || c == '-') {
        return Err(AppError::validation("Workspace url_key must be lowercase letters, numbers, or hyphens"));
    }
    if url_key.starts_with('-') || url_key.ends_with('-') {
        return Err(AppError::validation("Workspace url_key cannot start or end with hyphen"));
    }
    Ok(())
}


