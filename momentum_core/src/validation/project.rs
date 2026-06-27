use crate::error::AppError;

pub fn validate_create_project(name: &str, project_key: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::validation("Project name is required"));
    }
    if project_key.trim().is_empty() {
        return Err(AppError::validation("Project key is required"));
    }
    if project_key.len() > 10 {
        return Err(AppError::validation(
            "Project key must be 10 characters or less",
        ));
    }
    if !project_key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::validation(
            "Project key can only contain letters, numbers, hyphens, and underscores",
        ));
    }
    Ok(())
}
