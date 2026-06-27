use crate::db::models::workspace_member::WorkspaceMemberRole;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to invite members to a workspace
#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    pub emails: Vec<String>,
    pub role: Option<WorkspaceMemberRole>,
}

/// Information about an invitation
#[derive(Debug, Serialize)]
pub struct InvitationInfo {
    pub id: Uuid,
    pub email: String,
    pub role: WorkspaceMemberRole,
    pub status: crate::db::models::invitation::InvitationStatus,
    pub invited_by: Uuid,
    pub inviter_name: String,
    pub inviter_avatar_url: Option<String>,
    pub workspace_id: Uuid,
    pub workspace_name: String,
    pub workspace_logo_url: Option<String>,
    pub expires_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}