use serde::Deserialize;

/// Request to update a workspace
#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub url_key: Option<String>,
    pub logo_url: Option<String>,
}