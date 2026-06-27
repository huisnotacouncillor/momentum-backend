use crate::db::enums::LabelLevel;
use serde::Deserialize;

/// Request to create a new label
#[derive(Debug, Deserialize)]
pub struct CreateLabelRequest {
    pub name: String,
    pub color: String,
    pub level: LabelLevel,
}

/// Request to update an existing label
#[derive(Debug, Deserialize)]
pub struct UpdateLabelRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub level: Option<LabelLevel>,
}