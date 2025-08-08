use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Workspace models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::workspaces)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub url_key: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::workspaces)]
pub struct NewWorkspace {
    pub name: String,
    pub url_key: String,
}

// Workspace API DTOs
#[derive(Serialize)]
pub struct WorkspaceInfo {
    pub id: Uuid,
    pub name: String,
    pub url_key: String,
}

#[derive(Deserialize)]
pub struct SwitchWorkspaceRequest {
    pub workspace_id: Uuid,
}

#[derive(Serialize)]
pub struct WorkspaceSwitchResult {
    pub user_id: Uuid,
    pub previous_workspace_id: Option<Uuid>,
    pub current_workspace: WorkspaceInfo,
    pub user_role_in_workspace: String,
    pub available_teams: Vec<super::team::TeamInfo>,
}
