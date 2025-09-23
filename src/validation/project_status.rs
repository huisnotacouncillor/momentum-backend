use crate::db::models::project_status::ProjectStatusCategory;
use crate::error::AppError;

pub fn validate_create_project_status(name: &str, color: &Option<String>) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::validation("Status name is required"));
    }
    if let Some(c) = color {
        if !c.starts_with('#') || c.len() != 7 || !c.chars().skip(1).all(|x| x.is_ascii_hexdigit()) {
            return Err(AppError::validation("Color must be hex like #RRGGBB"));
        }
    }
    Ok(())
}

pub struct UpdateProjectStatusChanges<'a> {
    pub name: Option<&'a str>,
    pub description_present: bool,
    pub color: Option<&'a str>,
    pub category: Option<ProjectStatusCategory>,
}

pub fn validate_update_project_status(ch: &UpdateProjectStatusChanges) -> Result<(), AppError> {
    if ch.name.is_none() && !ch.description_present && ch.color.is_none() && ch.category.is_none() {
        return Err(AppError::validation("No update data provided"));
    }
    if let Some(n) = ch.name { if n.trim().is_empty() { return Err(AppError::validation("Status name cannot be empty")); } }
    if let Some(c) = ch.color { if !c.starts_with('#') || c.len() != 7 || !c.chars().skip(1).all(|x| x.is_ascii_hexdigit()) { return Err(AppError::validation("Color must be hex like #RRGGBB")); } }
    Ok(())
}


