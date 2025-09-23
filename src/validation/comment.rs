use crate::error::AppError;

pub fn validate_create_comment(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::validation("Comment content is required"));
    }

    if content.len() > 10000 {
        return Err(AppError::validation("Comment content is too long (max 10000 characters)"));
    }

    Ok(())
}

pub fn validate_update_comment(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::validation("Comment content cannot be empty"));
    }

    if content.len() > 10000 {
        return Err(AppError::validation("Comment content is too long (max 10000 characters)"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_comment_validation() {
        assert!(validate_create_comment("This is a valid comment").is_ok());
        assert!(validate_create_comment("").is_err());
        assert!(validate_create_comment("   ").is_err());
        assert!(validate_create_comment(&"a".repeat(10001)).is_err());
    }

    #[test]
    fn test_update_comment_validation() {
        assert!(validate_update_comment("Updated comment").is_ok());
        assert!(validate_update_comment("").is_err());
        assert!(validate_update_comment("   ").is_err());
        assert!(validate_update_comment(&"a".repeat(10001)).is_err());
    }
}
