//! Workspace member command handlers

use std::sync::Arc;
use uuid::Uuid;
use momentum_core::db::DbPool;
use momentum_core::db::models::workspace_member::WorkspaceMemberRole as DbWorkspaceMemberRole;
use momentum_core::error::AppError;
use momentum_core::services::context::RequestContext;
use momentum_core::services::workspace_members_service::WorkspaceMembersService;
use momentum_core::utils::AssetUrlHelper;
pub use momentum_core::services::workspace_members::types::{InviteMemberRequest, MembersAndInvitations};
use super::types::*;

pub struct WorkspaceMemberHandlers;

impl WorkspaceMemberHandlers {
    pub async fn handle_invite_member(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        data: InviteWorkspaceMemberCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let req = InviteMemberRequest {
            email: data.email,
            role: data.role.into(), // Convert to DbWorkspaceMemberRole
        };

        let invitation = WorkspaceMembersService::invite_member(&mut conn, &ctx, &req)?;

        Ok(serde_json::json!(invitation))
    }

    pub async fn handle_accept_invitation(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        invitation_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let invitation = WorkspaceMembersService::accept_invitation(&mut conn, &ctx, invitation_id)?;

        Ok(serde_json::json!(invitation))
    }

    pub async fn handle_list_workspace_members(
        db: &Arc<DbPool>,
        asset_helper: &AssetUrlHelper,
        ctx: RequestContext,
        filters: WorkspaceMemberFilters,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let members = WorkspaceMembersService::get_workspace_members(
            &mut conn,
            &ctx,
            asset_helper,
            ctx.workspace_id, // Use workspace from context
            filters.role.map(|r| r.into()), // Convert to DbWorkspaceMemberRole
            filters.user_id,
        )?;

        Ok(serde_json::json!(members))
    }
}

// Conversions from WebSocket command types to DB types
impl From<WorkspaceMemberRole> for DbWorkspaceMemberRole {
    fn from(role: WorkspaceMemberRole) -> Self {
        match role {
            WorkspaceMemberRole::Owner => DbWorkspaceMemberRole::Owner,
            WorkspaceMemberRole::Admin => DbWorkspaceMemberRole::Admin,
            WorkspaceMemberRole::Member => DbWorkspaceMemberRole::Member,
        }
    }
}
