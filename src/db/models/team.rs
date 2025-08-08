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
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::teams)]
pub struct NewTeam {
    pub workspace_id: Uuid,
    pub name: String,
    pub team_key: String,
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
    pub role: String,
}

#[derive(Serialize)]
pub struct TeamBasicInfo {
    pub id: Uuid,
    pub name: String,
    pub team_key: String,
}

#[derive(Serialize)]
pub struct TeamWithMembers {
    pub id: Uuid,
    pub name: String,
    pub team_key: String,
    pub members: Vec<TeamMemberInfo>,
}

#[derive(Serialize)]
pub struct TeamDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub team_key: String,
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