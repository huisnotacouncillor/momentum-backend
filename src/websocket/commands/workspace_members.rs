use uuid::Uuid;

use crate::{error::AppError, services::context::RequestContext};

use super::types::*;

pub struct WorkspaceMemberHandlers;

impl WorkspaceMemberHandlers {
    pub async fn handle_invite_member(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        data: InviteWorkspaceMemberCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let req = crate::routes::workspace_members::InviteMemberRequest {
            email: data.email,
            role: match data.role {
                WorkspaceMemberRole::Owner => {
                    crate::db::models::workspace_member::WorkspaceMemberRole::Owner
                }
                WorkspaceMemberRole::Admin => {
                    crate::db::models::workspace_member::WorkspaceMemberRole::Admin
                }
                WorkspaceMemberRole::Member => {
                    crate::db::models::workspace_member::WorkspaceMemberRole::Member
                }
            },
        };

        let invitation =
            crate::services::workspace_members_service::WorkspaceMembersService::invite_member(
                &mut conn, &ctx, &req,
            )?;

        Ok(serde_json::to_value(invitation).unwrap())
    }

    pub async fn handle_accept_invitation(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        invitation_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let invitation =
            crate::services::workspace_members_service::WorkspaceMembersService::accept_invitation(
                &mut conn,
                &ctx,
                invitation_id,
            )?;

        Ok(serde_json::to_value(invitation).unwrap())
    }

    pub async fn handle_list_workspace_members(
        db: &crate::db::DbPool,
        asset_helper: &crate::utils::AssetUrlHelper,
        ctx: RequestContext,
        filters: WorkspaceMemberFilters,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let role_enum = filters.role.map(|r| match r {
            WorkspaceMemberRole::Owner => {
                crate::db::models::workspace_member::WorkspaceMemberRole::Owner
            }
            WorkspaceMemberRole::Admin => {
                crate::db::models::workspace_member::WorkspaceMemberRole::Admin
            }
            WorkspaceMemberRole::Member => {
                crate::db::models::workspace_member::WorkspaceMemberRole::Member
            }
        });

        let members = crate::services::workspace_members_service::WorkspaceMembersService::get_current_workspace_members_with_search(
            &mut conn,
            &ctx,
            asset_helper,
            role_enum,
            filters.user_id,
            filters.search,
        )?;

        Ok(serde_json::to_value(members).unwrap())
    }
}
