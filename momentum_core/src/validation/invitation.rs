use crate::error::AppError;

pub fn validate_invite_email(email: &str) -> Result<(), AppError> {
    if email.trim().is_empty() {
        return Err(AppError::validation("Email is required"));
    }
    if !email.contains('@') || !email.contains('.') {
        return Err(AppError::validation("Invalid email format"));
    }
    Ok(())
}
