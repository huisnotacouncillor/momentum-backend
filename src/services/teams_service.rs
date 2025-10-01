use diesel::prelude::*;
use uuid::Uuid;

use crate::{
    db::models::team::{NewTeam, Team},
    error::AppError,
    schema,
    services::context::RequestContext,
};

pub struct TeamsService;

impl TeamsService {
    pub fn validate_name(name: &str) -> Result<(), AppError> {
        if name.trim().is_empty() {
            return Err(AppError::validation("Team name is required"));
        }
        Ok(())
    }

    pub fn validate_team_key(team_key: &str) -> Result<(), AppError> {
        if team_key.trim().is_empty() {
            return Err(AppError::validation("Team key is required"));
        }
        if !team_key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::validation(
                "Team key can only contain letters, numbers, hyphens, and underscores",
            ));
        }
        Ok(())
    }
    pub fn get(
        conn: &mut diesel::PgConnection,
        ctx: &RequestContext,
        team_id: Uuid,
    ) -> Result<Team, AppError> {
        use crate::schema::teams::dsl as t;
        match t::teams
            .filter(t::id.eq(team_id))
            .filter(t::workspace_id.eq(ctx.workspace_id))
            .select(Team::as_select())
            .first::<Team>(conn)
        {
            Ok(team) => Ok(team),
            Err(_) => Err(AppError::not_found("Team not found")),
        }
    }
    pub fn create(
        conn: &mut diesel::PgConnection,
        ctx: &RequestContext,
        req: &crate::routes::teams::CreateTeamRequest,
    ) -> Result<Team, AppError> {
        Self::validate_name(&req.name)?;
        Self::validate_team_key(&req.team_key)?;

        let user_id = ctx.user_id;
        let current_workspace_id = ctx.workspace_id;

        let result = conn.transaction::<Team, diesel::result::Error, _>(|conn| {
            let new_team = NewTeam {
                name: req.name.clone(),
                team_key: req.team_key.clone(),
                description: req.description.clone(),
                icon_url: req.icon_url.clone(),
                is_private: req.is_private,
                workspace_id: current_workspace_id,
            };

            let team: Team = diesel::insert_into(schema::teams::table)
                .values(&new_team)
                .get_result::<Team>(conn)?;

            let new_team_member = crate::db::models::team::NewTeamMember {
                user_id,
                team_id: team.id,
                role: "admin".to_string(),
            };

            diesel::insert_into(schema::team_members::table)
                .values(&new_team_member)
                .execute(conn)?;

            Ok(team)
        });

        match result {
            Ok(team) => Ok(team),
            Err(e) => {
                if e.to_string().contains("team_key") {
                    Err(AppError::validation(
                        "Team with this key already exists in this workspace",
                    ))
                } else {
                    Err(AppError::Internal("Failed to create team".to_string()))
                }
            }
        }
    }

    pub fn update(
        conn: &mut diesel::PgConnection,
        ctx: &RequestContext,
        team_id: Uuid,
        req: &crate::routes::teams::UpdateTeamRequest,
    ) -> Result<Team, AppError> {
        use crate::schema::teams::dsl as t;

        let existing_team = match t::teams
            .filter(t::id.eq(team_id))
            .filter(t::workspace_id.eq(ctx.workspace_id))
            .select(Team::as_select())
            .first::<Team>(conn)
        {
            Ok(team) => team,
            Err(_) => return Err(AppError::not_found("Team not found")),
        };

        if req.name.is_none()
            && req.team_key.is_none()
            && req.description.is_none()
            && req.icon_url.is_none()
            && req.is_private.is_none()
        {
            return Ok(existing_team);
        }

        let team_name = req.name.as_ref().unwrap_or(&existing_team.name);
        let team_key_val = req.team_key.as_ref().unwrap_or(&existing_team.team_key);
        let description_val = req
            .description
            .as_ref()
            .or(existing_team.description.as_ref());
        let icon_url_val = req.icon_url.as_ref().or(existing_team.icon_url.as_ref());
        let is_private_val = req.is_private.unwrap_or(existing_team.is_private);

        let updated = diesel::update(t::teams.filter(t::id.eq(team_id)))
            .set((
                t::name.eq(team_name),
                t::team_key.eq(team_key_val),
                t::description.eq(description_val),
                t::icon_url.eq(icon_url_val),
                t::is_private.eq(is_private_val),
            ))
            .get_result::<Team>(conn);

        match updated {
            Ok(team) => Ok(team),
            Err(e) => {
                if e.to_string().contains("team_key") {
                    Err(AppError::validation(
                        "Team with this key already exists in this workspace",
                    ))
                } else {
                    Err(AppError::Internal("Failed to update team".to_string()))
                }
            }
        }
    }

    pub fn delete(
        conn: &mut diesel::PgConnection,
        ctx: &RequestContext,
        team_id: Uuid,
    ) -> Result<(), AppError> {
        use crate::schema::teams::dsl as t;

        match t::teams
            .filter(t::id.eq(team_id))
            .filter(t::workspace_id.eq(ctx.workspace_id))
            .select(Team::as_select())
            .first::<Team>(conn)
        {
            Ok(_) => (),
            Err(_) => return Err(AppError::not_found("Team not found")),
        }

        match diesel::delete(t::teams.filter(t::id.eq(team_id))).execute(conn) {
            Ok(_) => Ok(()),
            Err(_) => Err(AppError::Internal("Failed to delete team".to_string())),
        }
    }

    pub fn list(
        conn: &mut diesel::PgConnection,
        ctx: &RequestContext,
    ) -> Result<Vec<Team>, AppError> {
        use crate::schema::teams::dsl as t;
        match t::teams
            .filter(t::workspace_id.eq(ctx.workspace_id))
            .select(Team::as_select())
            .order(t::created_at.desc())
            .load::<Team>(conn)
        {
            Ok(list) => Ok(list),
            Err(_) => Err(AppError::Internal("Failed to retrieve teams".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TeamsService;

    #[test]
    fn validate_name_rules() {
        assert!(TeamsService::validate_name("Alpha").is_ok());
        assert!(TeamsService::validate_name("  ").is_err());
        assert!(TeamsService::validate_name("").is_err());
    }

    #[test]
    fn validate_team_key_rules() {
        assert!(TeamsService::validate_team_key("team-1_ok").is_ok());
        assert!(TeamsService::validate_team_key("").is_err());
        assert!(TeamsService::validate_team_key("   ").is_err());
        assert!(TeamsService::validate_team_key("bad key").is_err());
        assert!(TeamsService::validate_team_key("bad@key").is_err());
    }
}
