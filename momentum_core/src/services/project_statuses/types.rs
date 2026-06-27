use serde::Deserialize;

/// Request to update a project status
#[derive(Debug, Deserialize)]
pub struct UpdateProjectStatusRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: Option<String>,
}