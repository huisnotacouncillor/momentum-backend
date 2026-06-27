use serde::Deserialize;

/// Request to create a new team
#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
}

/// Request to update an existing team
#[derive(Debug, Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub team_key: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: Option<bool>,
}