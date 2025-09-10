use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Team models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::teams)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Team {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub team_key: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::teams)]
pub struct NewTeam {
    pub workspace_id: Uuid,
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
}

// Team Member models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::team_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TeamMember {
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::team_members)]
pub struct NewTeamMember {
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub role: String,
}

// Team API DTOs
#[derive(Serialize)]
pub struct TeamInfo {
    pub id: Uuid,
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
    pub role: String,
}

#[derive(Serialize, Clone)]
pub struct TeamBasicInfo {
    pub id: Uuid,
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
}

#[derive(Serialize)]
pub struct TeamWithMembers {
    pub id: Uuid,
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
    pub members: Vec<TeamMemberInfo>,
}

#[derive(Serialize)]
pub struct TeamDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
    pub workspace_id: Uuid,
    pub members: Vec<TeamMemberInfo>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct TeamMemberInfo {
    pub user: crate::db::models::auth::UserBasicInfo,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
}

#[derive(Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub team_key: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: Option<bool>,
}
