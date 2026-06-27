use crate::error::AppError;

pub fn validate_create_workflow(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::validation("Workflow name is required"));
    }
    Ok(())
}

pub fn validate_create_state(name: &str, position: i32) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::validation("State name is required"));
    }
    if position < 0 {
        return Err(AppError::validation("State position must be non-negative"));
    }
    Ok(())
}
