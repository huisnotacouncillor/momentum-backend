use uuid::Uuid;

use crate::{error::AppError, services::context::RequestContext};

use super::types::*;

pub struct TeamHandlers;

impl TeamHandlers {
    pub async fn handle_create_team(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        data: CreateTeamCommand,
    ) -> Result<serde_json::Value, AppError> {
        if data.name.trim().is_empty() {
            return Err(AppError::validation("Team name is required"));
        }
        if data.team_key.trim().is_empty() {
            return Err(AppError::validation("Team key is required"));
        }
        if !data
            .team_key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::validation(
                "Team key can only contain letters, numbers, hyphens, and underscores",
            ));
        }
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let req = crate::routes::teams::CreateTeamRequest {
            name: data.name,
            team_key: data.team_key,
            description: data.description,
            icon_url: data.icon_url,
            is_private: data.is_private,
        };
        let team = crate::services::teams_service::TeamsService::create(&mut conn, &ctx, &req)?;
        Ok(serde_json::to_value(team).unwrap())
    }

    pub async fn handle_update_team(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        team_id: Uuid,
        data: UpdateTeamCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let req = crate::routes::teams::UpdateTeamRequest {
            name: data.name,
            team_key: data.team_key,
            description: data.description,
            icon_url: data.icon_url,
            is_private: data.is_private,
        };
        let team =
            crate::services::teams_service::TeamsService::update(&mut conn, &ctx, team_id, &req)?;
        Ok(serde_json::to_value(team).unwrap())
    }

    pub async fn handle_delete_team(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        team_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        crate::services::teams_service::TeamsService::delete(&mut conn, &ctx, team_id)?;
        Ok(serde_json::json!({"deleted": true, "team_id": team_id}))
    }

    pub async fn handle_query_teams(
        db: &crate::db::DbPool,
        ctx: RequestContext,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let list = crate::services::teams_service::TeamsService::list(&mut conn, &ctx)?;
        Ok(serde_json::to_value(list).unwrap())
    }
}
