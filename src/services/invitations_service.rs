use diesel::prelude::*;

use crate::{
    db::models::invitation::{Invitation, NewInvitation, InvitationStatus},
    db::repositories::invitations::InvitationsRepo,
    db::repositories::workspace_members::WorkspaceMembersRepo,
    error::AppError,
    services::context::RequestContext,
};

pub struct InvitationsService;

impl InvitationsService {
    pub fn create(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        email: &str,
        role: crate::db::models::workspace_member::WorkspaceMemberRole,
    ) -> Result<Invitation, AppError> {
        if InvitationsRepo::pending_exists_for_email(conn, ctx.workspace_id, email)? {
            return Err(AppError::conflict_with_code("User already has a pending invitation to this workspace", Some("email".into()), "PENDING_INVITATION"));
        }
        let new_inv = NewInvitation {
            workspace_id: ctx.workspace_id,
            email: email.to_string(),
            role,
            invited_by: ctx.user_id,
        };
        let inv = InvitationsRepo::insert(conn, &new_inv)?;
        Ok(inv)
    }

    pub fn accept(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        invitation_id: uuid::Uuid,
    ) -> Result<Invitation, AppError> {
        // ensure invitation belongs to email of current user? callsites should check
        let updated = conn.transaction::<Invitation, diesel::result::Error, _>(|tx| {
            let inv = InvitationsRepo::update_status(tx, invitation_id, InvitationStatus::Accepted)?;
            // add workspace member
            let new_member = crate::db::models::workspace_member::NewWorkspaceMember { user_id: ctx.user_id, workspace_id: inv.workspace_id, role: inv.role.clone() };
            let _ = WorkspaceMembersRepo::insert(tx, &new_member)?;
            Ok(inv)
        })?;
        Ok(updated)
    }

    pub fn invite_members(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        req: &crate::routes::invitations::InviteMemberRequest,
    ) -> Result<Vec<Invitation>, AppError> {
        let mut invitations = Vec::new();
        let role = req
            .role
            .clone()
            .unwrap_or(crate::db::models::workspace_member::WorkspaceMemberRole::Member);

        for email in &req.emails {
            let invitation = Self::create(conn, ctx, email, role.clone())?;
            invitations.push(invitation);
        }

        Ok(invitations)
    }

    pub fn get_user_invitations(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
        status: Option<crate::db::models::invitation::InvitationStatus>,
        email: Option<String>,
    ) -> Result<Vec<crate::routes::invitations::InvitationInfo>, AppError> {
        let mut invitations = InvitationsRepo::list_by_workspace(conn, ctx.workspace_id)?;

        // Apply filters
        if let Some(status_filter) = status {
            invitations.retain(|inv| inv.status == status_filter);
        }
        if let Some(email_filter) = email {
            invitations.retain(|inv| inv.email == email_filter);
        }

        // Get workspace info
        let workspace = crate::schema::workspaces::table
            .filter(crate::schema::workspaces::id.eq(ctx.workspace_id))
            .select(crate::db::models::workspace::Workspace::as_select())
            .first::<crate::db::models::workspace::Workspace>(conn)
            .optional()?
            .ok_or_else(|| AppError::internal("Failed to retrieve workspace"))?;

        let mut invitation_infos = Vec::new();
        for invitation in invitations {
            // Get inviter info
            let inviter = crate::schema::users::table
                .filter(crate::schema::users::id.eq(invitation.invited_by))
                .select(crate::db::models::auth::User::as_select())
                .first::<crate::db::models::auth::User>(conn)
                .optional()?
                .ok_or_else(|| AppError::internal("Failed to retrieve inviter"))?;

            let processed_avatar_url = inviter
                .avatar_url
                .as_ref()
                .map(|url| asset_helper.process_url(url));

            invitation_infos.push(crate::routes::invitations::InvitationInfo {
                id: invitation.id,
                email: invitation.email,
                role: invitation.role,
                status: invitation.status,
                invited_by: invitation.invited_by,
                inviter_name: inviter.username,
                inviter_avatar_url: processed_avatar_url,
                workspace_id: invitation.workspace_id,
                workspace_name: workspace.name.clone(),
                workspace_logo_url: workspace.logo_url.clone(),
                expires_at: invitation.expires_at.naive_utc(),
                created_at: invitation.created_at.naive_utc(),
                updated_at: invitation.updated_at.naive_utc(),
            });
        }

        Ok(invitation_infos)
    }

    pub fn get_by_id(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
        invitation_id: uuid::Uuid,
    ) -> Result<crate::routes::invitations::InvitationInfo, AppError> {
        let invitation = InvitationsRepo::find_by_id(conn, invitation_id)?
            .ok_or_else(|| AppError::not_found("invitation"))?;

        // Verify invitation belongs to workspace
        if invitation.workspace_id != ctx.workspace_id {
            return Err(AppError::not_found("invitation"));
        }

        // Get inviter info
        let inviter = crate::schema::users::table
            .filter(crate::schema::users::id.eq(invitation.invited_by))
            .select(crate::db::models::auth::User::as_select())
            .first::<crate::db::models::auth::User>(conn)
            .optional()?
            .ok_or_else(|| AppError::internal("Failed to retrieve inviter"))?;

        // Get workspace info
        let workspace = crate::schema::workspaces::table
            .filter(crate::schema::workspaces::id.eq(invitation.workspace_id))
            .select(crate::db::models::workspace::Workspace::as_select())
            .first::<crate::db::models::workspace::Workspace>(conn)
            .optional()?
            .ok_or_else(|| AppError::internal("Failed to retrieve workspace"))?;

        let processed_avatar_url = inviter.avatar_url.as_ref().map(|url| asset_helper.process_url(url));

        Ok(crate::routes::invitations::InvitationInfo {
            id: invitation.id,
            email: invitation.email,
            role: invitation.role,
            status: invitation.status,
            invited_by: invitation.invited_by,
            inviter_name: inviter.username,
            inviter_avatar_url: processed_avatar_url,
            workspace_id: invitation.workspace_id,
            workspace_name: workspace.name,
            workspace_logo_url: workspace.logo_url,
            expires_at: invitation.expires_at.naive_utc(),
            created_at: invitation.created_at.naive_utc(),
            updated_at: invitation.updated_at.naive_utc(),
        })
    }

    pub fn decline(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        invitation_id: uuid::Uuid,
    ) -> Result<Invitation, AppError> {
        let invitation = InvitationsRepo::find_by_id(conn, invitation_id)?
            .ok_or_else(|| AppError::not_found("invitation"))?;

        // Verify invitation belongs to workspace
        if invitation.workspace_id != ctx.workspace_id {
            return Err(AppError::not_found("invitation"));
        }

        let updated = InvitationsRepo::update_status(conn, invitation_id, InvitationStatus::Declined)?;
        Ok(updated)
    }

    pub fn revoke(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        invitation_id: uuid::Uuid,
    ) -> Result<(), AppError> {
        let invitation = InvitationsRepo::find_by_id(conn, invitation_id)?
            .ok_or_else(|| AppError::not_found("invitation"))?;

        // Verify invitation belongs to workspace
        if invitation.workspace_id != ctx.workspace_id {
            return Err(AppError::not_found("invitation"));
        }

        InvitationsRepo::delete_by_id(conn, invitation_id)?;
        Ok(())
    }
}


