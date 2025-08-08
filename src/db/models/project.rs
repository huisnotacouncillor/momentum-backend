use crate::db::enums::ProjectStatus;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Project models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::projects)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Project {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub roadmap_id: Option<Uuid>,
    pub owner_id: Uuid,
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub target_date: Option<chrono::NaiveDate>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::projects)]
pub struct NewProject {
    pub workspace_id: Uuid,
    pub roadmap_id: Option<Uuid>,
    pub owner_id: Uuid,
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub target_date: Option<chrono::NaiveDate>,
}

// Project API DTOs
#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub roadmap_id: Option<Uuid>,
    pub target_date: Option<chrono::NaiveDate>,
}

#[derive(Serialize)]
pub struct ProjectInfo {
    pub id: Uuid,
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub target_date: Option<chrono::NaiveDate>,
    pub owner: super::auth::UserBasicInfo,
    pub teams: Vec<super::team::TeamBasicInfo>,
    pub workspace_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectInfo>,
    pub total_count: i64,
}

#[derive(Deserialize)]
pub struct ProjectListQuery {
    pub workspace_id: Option<Uuid>,
    pub status: Option<ProjectStatus>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}