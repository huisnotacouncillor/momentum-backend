use crate::error::AppError;

pub fn validate_create_label(name: &str, color: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::validation("Label name is required"));
    }
    if !color.starts_with('#')
        || color.len() != 7
        || !color.chars().skip(1).all(|c| c.is_ascii_hexdigit())
    {
        return Err(AppError::validation("Color must be hex like #RRGGBB"));
    }
    Ok(())
}

pub struct UpdateLabelChanges<'a> {
    pub name: Option<&'a str>,
    pub color: Option<&'a str>,
    pub level_present: bool,
}

pub fn validate_update_label(changes: &UpdateLabelChanges) -> Result<(), AppError> {
    if changes.name.is_none() && changes.color.is_none() && !changes.level_present {
        return Err(AppError::validation("No update data provided"));
    }
    if let Some(name) = changes.name {
        if name.trim().is_empty() {
            return Err(AppError::validation("Label name cannot be empty"));
        }
    }
    if let Some(color) = changes.color {
        if !color.starts_with('#')
            || color.len() != 7
            || !color.chars().skip(1).all(|c| c.is_ascii_hexdigit())
        {
            return Err(AppError::validation("Color must be hex like #RRGGBB"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_validation() {
        assert!(validate_create_label("bug", "#FF00AA").is_ok());
        assert!(validate_create_label(" ", "#FF00AA").is_err());
        assert!(validate_create_label("bug", "123456").is_err());
    }

    #[test]
    fn test_update_validation() {
        // no fields
        let c = UpdateLabelChanges {
            name: None,
            color: None,
            level_present: false,
        };
        assert!(validate_update_label(&c).is_err());

        // empty name
        let c = UpdateLabelChanges {
            name: Some("  "),
            color: None,
            level_present: false,
        };
        assert!(validate_update_label(&c).is_err());

        // bad color
        let c = UpdateLabelChanges {
            name: None,
            color: Some("red"),
            level_present: false,
        };
        assert!(validate_update_label(&c).is_err());

        // valid name
        let c = UpdateLabelChanges {
            name: Some("feature"),
            color: None,
            level_present: false,
        };
        assert!(validate_update_label(&c).is_ok());

        // valid color
        let c = UpdateLabelChanges {
            name: None,
            color: Some("#00FF00"),
            level_present: false,
        };
        assert!(validate_update_label(&c).is_ok());

        // valid level only
        let c = UpdateLabelChanges {
            name: None,
            color: None,
            level_present: true,
        };
        assert!(validate_update_label(&c).is_ok());
    }
}
