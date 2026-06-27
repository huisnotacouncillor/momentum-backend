use crate::error::AppError;

pub fn validate_register_request(
    name: &str,
    username: &str,
    email: &str,
    password: &str,
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::validation("Name is required"));
    }

    if username.trim().is_empty() {
        return Err(AppError::validation("Username is required"));
    }

    if username.len() < 3 {
        return Err(AppError::validation(
            "Username must be at least 3 characters",
        ));
    }

    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::validation(
            "Username can only contain letters, numbers, and underscores",
        ));
    }

    if email.trim().is_empty() {
        return Err(AppError::validation("Email is required"));
    }

    if !email.contains('@') || !email.contains('.') {
        return Err(AppError::validation("Invalid email format"));
    }

    if password.len() < 8 {
        return Err(AppError::validation(
            "Password must be at least 8 characters",
        ));
    }

    Ok(())
}

pub fn validate_login_request(email: &str, password: &str) -> Result<(), AppError> {
    if email.trim().is_empty() {
        return Err(AppError::validation("Email is required"));
    }

    if password.trim().is_empty() {
        return Err(AppError::validation("Password is required"));
    }

    Ok(())
}

pub struct UpdateProfileChanges<'a> {
    pub name: Option<&'a str>,
    pub username: Option<&'a str>,
    pub email: Option<&'a str>,
    pub avatar_url: Option<&'a str>,
}

pub fn validate_update_profile(changes: &UpdateProfileChanges) -> Result<(), AppError> {
    if changes.name.is_none()
        && changes.username.is_none()
        && changes.email.is_none()
        && changes.avatar_url.is_none()
    {
        return Err(AppError::validation("No update data provided"));
    }

    if let Some(name) = changes.name {
        if name.trim().is_empty() {
            return Err(AppError::validation("Name cannot be empty"));
        }
    }

    if let Some(username) = changes.username {
        if username.trim().is_empty() {
            return Err(AppError::validation("Username cannot be empty"));
        }
        if username.len() < 3 {
            return Err(AppError::validation(
                "Username must be at least 3 characters",
            ));
        }
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(AppError::validation(
                "Username can only contain letters, numbers, and underscores",
            ));
        }
    }

    if let Some(email) = changes.email {
        if email.trim().is_empty() {
            return Err(AppError::validation("Email cannot be empty"));
        }
        if !email.contains('@') || !email.contains('.') {
            return Err(AppError::validation("Invalid email format"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_validation() {
        assert!(
            validate_register_request("John Doe", "johndoe", "john@example.com", "password123")
                .is_ok()
        );
        assert!(
            validate_register_request("", "johndoe", "john@example.com", "password123").is_err()
        );
        assert!(
            validate_register_request("John Doe", "jo", "john@example.com", "password123").is_err()
        );
        assert!(
            validate_register_request("John Doe", "johndoe", "invalid-email", "password123")
                .is_err()
        );
        assert!(
            validate_register_request("John Doe", "johndoe", "john@example.com", "123").is_err()
        );
    }

    #[test]
    fn test_login_validation() {
        assert!(validate_login_request("john@example.com", "password123").is_ok());
        assert!(validate_login_request("", "password123").is_err());
        assert!(validate_login_request("john@example.com", "").is_err());
    }

    #[test]
    fn test_update_profile_validation() {
        let changes = UpdateProfileChanges {
            name: Some("John Doe"),
            username: Some("johndoe"),
            email: Some("john@example.com"),
            avatar_url: Some("https://example.com/avatar.jpg"),
        };
        assert!(validate_update_profile(&changes).is_ok());

        let empty_changes = UpdateProfileChanges {
            name: None,
            username: None,
            email: None,
            avatar_url: None,
        };
        assert!(validate_update_profile(&empty_changes).is_err());

        let invalid_changes = UpdateProfileChanges {
            name: Some(""),
            username: Some("jo"),
            email: Some("invalid-email"),
            avatar_url: None,
        };
        assert!(validate_update_profile(&invalid_changes).is_err());
    }
}
