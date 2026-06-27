use serde::Deserialize;

/// Request to update user profile
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}