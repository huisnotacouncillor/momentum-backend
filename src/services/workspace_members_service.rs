use diesel::prelude::*;

use crate::{
    db::models::workspace_member::{NewWorkspaceMember, WorkspaceMember},
    db::repositories::workspace_members::WorkspaceMembersRepo,
    error::AppError,
    services::context::RequestContext,
};

pub struct WorkspaceMembersService;

impl WorkspaceMembersService {
    pub fn list(
        conn: &mut PgConnection,
        ctx: &RequestContext,
    ) -> Result<Vec<WorkspaceMember>, AppError> {
        let list = WorkspaceMembersRepo::list_by_workspace(conn, ctx.workspace_id)?;
        Ok(list)
    }

    pub fn add(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        user_id: uuid::Uuid,
        role: crate::db::models::workspace_member::WorkspaceMemberRole,
    ) -> Result<WorkspaceMember, AppError> {
        if WorkspaceMembersRepo::find(conn, ctx.workspace_id, user_id)?.is_some() {
            return Err(AppError::conflict_with_code(
                "User already a member",
                None,
                "ALREADY_MEMBER",
            ));
        }
        let new_member = NewWorkspaceMember {
            user_id,
            workspace_id: ctx.workspace_id,
            role,
        };
        let member = WorkspaceMembersRepo::insert(conn, &new_member)?;
        Ok(member)
    }

    pub fn invite_member(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        req: &crate::routes::workspace_members::InviteMemberRequest,
    ) -> Result<WorkspaceMember, AppError> {
        // Check if user already exists
        let existing_user = crate::schema::users::table
            .filter(crate::schema::users::email.eq(&req.email))
            .first::<crate::db::models::auth::User>(conn)
            .optional()?;

        if let Some(user) = existing_user {
            // User exists, add them directly
            if WorkspaceMembersRepo::find(conn, ctx.workspace_id, user.id)?.is_some() {
                return Err(AppError::conflict_with_code(
                    "User already a member",
                    None,
                    "ALREADY_MEMBER",
                ));
            }
            let new_member = NewWorkspaceMember {
                user_id: user.id,
                workspace_id: ctx.workspace_id,
                role: req.role.clone(),
            };
            let member = WorkspaceMembersRepo::insert(conn, &new_member)?;
            Ok(member)
        } else {
            // User doesn't exist, create invitation
            let _invitation = crate::services::InvitationsService::create(
                conn,
                ctx,
                &req.email,
                req.role.clone(),
            )?;
            // Return a placeholder member - in real implementation, this might be different
            Err(AppError::conflict_with_code(
                "User doesn't exist, invitation sent",
                None,
                "INVITATION_SENT",
            ))
        }
    }

    pub fn accept_invitation(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        invitation_id: uuid::Uuid,
    ) -> Result<WorkspaceMember, AppError> {
        let _invitation = crate::services::InvitationsService::accept(conn, ctx, invitation_id)?;

        // Get the newly created member
        let member = WorkspaceMembersRepo::find(conn, ctx.workspace_id, ctx.user_id)?
            .ok_or_else(|| AppError::internal("Failed to retrieve workspace member"))?;

        Ok(member)
    }

    pub fn get_workspace_members(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
        workspace_id: uuid::Uuid,
        role: Option<crate::db::models::workspace_member::WorkspaceMemberRole>,
        user_id: Option<uuid::Uuid>,
    ) -> Result<Vec<crate::routes::workspace_members::WorkspaceMemberInfo>, AppError> {
        Self::get_workspace_members_with_search(
            conn,
            _ctx,
            asset_helper,
            workspace_id,
            role,
            user_id,
            None,
        )
    }

    pub fn get_workspace_members_with_search(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
        workspace_id: uuid::Uuid,
        role: Option<crate::db::models::workspace_member::WorkspaceMemberRole>,
        user_id: Option<uuid::Uuid>,
        search: Option<String>,
    ) -> Result<Vec<crate::routes::workspace_members::WorkspaceMemberInfo>, AppError> {
        let mut members = WorkspaceMembersRepo::list_by_workspace(conn, workspace_id)?;

        // Apply filters
        if let Some(role_filter) = role {
            members.retain(|member| member.role == role_filter);
        }
        if let Some(user_filter) = user_id {
            members.retain(|member| member.user_id == user_filter);
        }

        let mut member_infos = Vec::new();
        for member in members {
            let user = crate::schema::users::table
                .filter(crate::schema::users::id.eq(member.user_id))
                .select(crate::db::models::auth::User::as_select())
                .first::<crate::db::models::auth::User>(conn)
                .optional()?
                .ok_or_else(|| AppError::internal("Failed to retrieve user"))?;

            // Apply search filter
            if let Some(ref search_term) = search {
                let search_lower = search_term.to_lowercase();
                let name_match = user.name.to_lowercase().contains(&search_lower);
                let username_match = user.username.to_lowercase().contains(&search_lower);
                let email_match = user.email.to_lowercase().contains(&search_lower);

                if !name_match && !username_match && !email_match {
                    continue;
                }
            }

            let processed_avatar_url = user
                .avatar_url
                .as_ref()
                .map(|url| asset_helper.process_url(url));
            let user_basic = crate::db::models::auth::UserBasicInfo {
                id: user.id,
                name: user.name,
                username: user.username,
                email: user.email,
                avatar_url: processed_avatar_url,
            };

            member_infos.push(crate::routes::workspace_members::WorkspaceMemberInfo {
                id: member.user_id,
                user_id: member.user_id,
                workspace_id: member.workspace_id,
                user: user_basic,
                role: member.role,
                created_at: member.created_at.naive_utc(),
                updated_at: member.updated_at.naive_utc(),
            });
        }

        Ok(member_infos)
    }

    pub fn get_members_and_invitations(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
        role: Option<crate::db::models::workspace_member::WorkspaceMemberRole>,
        user_id: Option<uuid::Uuid>,
    ) -> Result<crate::routes::workspace_members::MembersAndInvitations, AppError> {
        // Get members
        let members =
            Self::get_workspace_members(conn, ctx, asset_helper, ctx.workspace_id, role, user_id)?;

        // Get invitations
        let invitations = crate::services::InvitationsService::get_user_invitations(
            conn,
            ctx,
            asset_helper,
            Some(crate::db::models::invitation::InvitationStatus::Pending),
            None,
        )?;

        Ok(crate::routes::workspace_members::MembersAndInvitations {
            members,
            invitations,
        })
    }

    pub fn get_current_workspace_members(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
        role: Option<crate::db::models::workspace_member::WorkspaceMemberRole>,
        user_id: Option<uuid::Uuid>,
    ) -> Result<Vec<crate::routes::workspace_members::WorkspaceMemberInfo>, AppError> {
        Self::get_workspace_members(conn, ctx, asset_helper, ctx.workspace_id, role, user_id)
    }

    pub fn get_current_workspace_members_with_search(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
        role: Option<crate::db::models::workspace_member::WorkspaceMemberRole>,
        user_id: Option<uuid::Uuid>,
        search: Option<String>,
    ) -> Result<Vec<crate::routes::workspace_members::WorkspaceMemberInfo>, AppError> {
        Self::get_workspace_members_with_search(
            conn,
            ctx,
            asset_helper,
            ctx.workspace_id,
            role,
            user_id,
            search,
        )
    }
}
