use crate::db::models::workspace_member::WorkspaceMemberRole;
use crate::db::models::auth::UserBasicInfo;
use chrono::NaiveDateTime;
use serde::Serialize;
use uuid::Uuid;

/// Request to invite a member to a workspace
#[derive(Debug)]
pub struct InviteMemberRequest {
    pub email: String,
    pub role: WorkspaceMemberRole,
}

/// Information about a workspace member
#[derive(Debug, Serialize)]
pub struct WorkspaceMemberInfo {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub role: WorkspaceMemberRole,
    pub user: UserBasicInfo,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Combined members and invitations response
#[derive(Debug, Serialize)]
pub struct MembersAndInvitations {
    pub members: Vec<WorkspaceMemberInfo>,
    pub invitations: Vec<crate::services::invitations::types::InvitationInfo>,
}