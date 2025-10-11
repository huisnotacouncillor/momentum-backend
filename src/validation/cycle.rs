use crate::error::AppError;

pub fn validate_create_cycle(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::validation("Cycle name is required"));
    }
    Ok(())
}
