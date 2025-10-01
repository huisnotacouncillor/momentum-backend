use diesel::prelude::*;
use uuid::Uuid;

use crate::{
    db::models::auth::User,
    db::models::team::{NewTeamMember, Team, TeamMember},
    error::AppError,
    schema,
    services::context::RequestContext,
};

pub struct TeamMembersService;

impl TeamMembersService {
    pub fn normalize_role(role: &str) -> Result<&'static str, AppError> {
        match role {
            "admin" => Ok("admin"),
            "member" => Ok("member"),
            _ => Err(AppError::validation("Invalid team member role")),
        }
    }
    pub fn list(
        conn: &mut diesel::PgConnection,
        ctx: &RequestContext,
        team_id: Uuid,
    ) -> Result<Vec<(TeamMember, User)>, AppError> {
        use crate::schema::teams::dsl as t;
        // Ensure team in workspace
        t::teams
            .filter(t::id.eq(team_id))
            .filter(t::workspace_id.eq(ctx.workspace_id))
            .select(Team::as_select())
            .first::<Team>(conn)
            .map_err(|_| AppError::not_found("Team not found"))?;

        let members = schema::team_members::table
            .filter(schema::team_members::team_id.eq(team_id))
            .inner_join(
                schema::users::table.on(schema::users::id.eq(schema::team_members::user_id)),
            )
            .select((TeamMember::as_select(), User::as_select()))
            .load::<(TeamMember, User)>(conn)
            .map_err(|_| AppError::internal("Failed to retrieve team members"))?;

        Ok(members)
    }

    pub fn add(
        conn: &mut diesel::PgConnection,
        ctx: &RequestContext,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<(), AppError> {
        let role = Self::normalize_role(role)?;
        use crate::schema::teams::dsl as t;
        // Ensure team in workspace
        t::teams
            .filter(t::id.eq(team_id))
            .filter(t::workspace_id.eq(ctx.workspace_id))
            .select(Team::as_select())
            .first::<Team>(conn)
            .map_err(|_| AppError::not_found("Team not found"))?;

        // Ensure user is workspace member
        use crate::schema::workspace_members::dsl as wm;
        schema::workspace_members::table
            .filter(wm::user_id.eq(user_id))
            .filter(wm::workspace_id.eq(ctx.workspace_id))
            .select(crate::db::models::workspace_member::WorkspaceMember::as_select())
            .first::<crate::db::models::workspace_member::WorkspaceMember>(conn)
            .map_err(|_| AppError::validation("User is not a member of this workspace"))?;

        let new_member = NewTeamMember {
            user_id,
            team_id,
            role: role.to_string(),
        };
        diesel::insert_into(schema::team_members::table)
            .values(&new_member)
            .execute(conn)
            .map_err(|e| {
                if e.to_string().contains("team_members_user_id_team_id_key") {
                    AppError::validation("User is already a member of this team")
                } else {
                    AppError::internal("Failed to add team member")
                }
            })?;
        Ok(())
    }

    pub fn update(
        conn: &mut diesel::PgConnection,
        ctx: &RequestContext,
        team_id: Uuid,
        member_user_id: Uuid,
        role: &str,
    ) -> Result<(), AppError> {
        let role = Self::normalize_role(role)?;
        use crate::schema::teams::dsl as t;
        t::teams
            .filter(t::id.eq(team_id))
            .filter(t::workspace_id.eq(ctx.workspace_id))
            .select(Team::as_select())
            .first::<Team>(conn)
            .map_err(|_| AppError::not_found("Team not found"))?;

        use crate::schema::team_members::dsl as tm;
        schema::team_members::table
            .filter(tm::team_id.eq(team_id))
            .filter(tm::user_id.eq(member_user_id))
            .select(TeamMember::as_select())
            .first::<TeamMember>(conn)
            .map_err(|_| AppError::not_found("Team member not found"))?;

        diesel::update(
            schema::team_members::table
                .filter(tm::team_id.eq(team_id))
                .filter(tm::user_id.eq(member_user_id)),
        )
        .set(schema::team_members::role.eq(role.to_string()))
        .execute(conn)
        .map_err(|_| AppError::internal("Failed to update team member"))?;
        Ok(())
    }

    pub fn remove(
        conn: &mut diesel::PgConnection,
        ctx: &RequestContext,
        team_id: Uuid,
        member_user_id: Uuid,
    ) -> Result<(), AppError> {
        use crate::schema::teams::dsl as t;
        t::teams
            .filter(t::id.eq(team_id))
            .filter(t::workspace_id.eq(ctx.workspace_id))
            .select(Team::as_select())
            .first::<Team>(conn)
            .map_err(|_| AppError::not_found("Team not found"))?;

        use crate::schema::team_members::dsl as tm;
        schema::team_members::table
            .filter(tm::team_id.eq(team_id))
            .filter(tm::user_id.eq(member_user_id))
            .select(TeamMember::as_select())
            .first::<TeamMember>(conn)
            .map_err(|_| AppError::not_found("Team member not found"))?;

        diesel::delete(
            schema::team_members::table
                .filter(tm::team_id.eq(team_id))
                .filter(tm::user_id.eq(member_user_id)),
        )
        .execute(conn)
        .map_err(|_| AppError::internal("Failed to remove team member"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::TeamMembersService;

    #[test]
    fn normalize_role_accepts_valid_roles() {
        assert_eq!(
            TeamMembersService::normalize_role("admin").unwrap(),
            "admin"
        );
        assert_eq!(
            TeamMembersService::normalize_role("member").unwrap(),
            "member"
        );
    }

    #[test]
    fn normalize_role_rejects_invalid_roles() {
        assert!(TeamMembersService::normalize_role("").is_err());
        assert!(TeamMembersService::normalize_role("owner").is_err());
        assert!(TeamMembersService::normalize_role("ADMIN").is_err());
    }
}
