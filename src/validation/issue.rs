use crate::db::enums::IssuePriority;
use crate::error::AppError;

pub fn validate_create_issue(
    title: &str,
    description: &Option<String>,
    _team_id: &uuid::Uuid,
) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::validation("Issue title is required"));
    }

    if title.len() > 255 {
        return Err(AppError::validation(
            "Issue title is too long (max 255 characters)",
        ));
    }

    if let Some(desc) = description {
        if desc.len() > 10000 {
            return Err(AppError::validation(
                "Issue description is too long (max 10000 characters)",
            ));
        }
    }

    Ok(())
}

pub fn validate_update_issue(
    title: &Option<String>,
    description: &Option<String>,
) -> Result<(), AppError> {
    if title.is_none() && description.is_none() {
        return Err(AppError::validation("No update data provided"));
    }

    if let Some(title) = title {
        if title.trim().is_empty() {
            return Err(AppError::validation("Issue title cannot be empty"));
        }

        if title.len() > 255 {
            return Err(AppError::validation(
                "Issue title is too long (max 255 characters)",
            ));
        }
    }

    if let Some(description) = description {
        if description.len() > 10000 {
            return Err(AppError::validation(
                "Issue description is too long (max 10000 characters)",
            ));
        }
    }

    Ok(())
}

pub fn validate_priority(_priority: &Option<IssuePriority>) -> Result<(), AppError> {
    // Priority validation is handled by the enum type itself
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_create_issue_validation() {
        let team_id = Uuid::new_v4();

        assert!(validate_create_issue("Valid title", &None, &team_id).is_ok());
        assert!(validate_create_issue("", &None, &team_id).is_err());
        assert!(validate_create_issue(&"a".repeat(256), &None, &team_id).is_err());
        assert!(validate_create_issue("Valid title", &Some("a".repeat(10001)), &team_id).is_err());
    }

    #[test]
    fn test_update_issue_validation() {
        assert!(validate_update_issue(&Some("Valid title".to_string()), &None).is_ok());
        assert!(validate_update_issue(&None, &Some("Valid description".to_string())).is_ok());
        assert!(validate_update_issue(&None, &None).is_err());
        assert!(validate_update_issue(&Some("".to_string()), &None).is_err());
        assert!(validate_update_issue(&Some("a".repeat(256)), &None).is_err());
    }
}
