use uuid::Uuid;

use crate::{error::AppError, services::context::RequestContext, utils::AssetUrlHelper};

use super::types::*;

pub struct ProjectHandlers;

impl ProjectHandlers {
    pub async fn handle_create_project(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        data: CreateProjectCommand,
        _asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        if data.name.trim().is_empty() {
            return Err(AppError::validation("Project name is required"));
        }
        if data.project_key.trim().is_empty() {
            return Err(AppError::validation("Project key is required"));
        }
        if !data
            .project_key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::validation(
                "Project key can only contain letters, numbers, hyphens, and underscores",
            ));
        }

        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let create_req = crate::db::models::project::CreateProjectRequest {
            name: data.name,
            project_key: data.project_key,
            description: data.description,
            target_date: data.target_date,
            project_status_id: data.project_status_id,
            priority: data.priority.map(|p| p.parse().unwrap_or_default()),
            roadmap_id: None,
        };

        let project = crate::services::projects_service::ProjectsService::create(
            &mut conn,
            &ctx,
            &create_req,
        )?;
        Ok(serde_json::to_value(project).unwrap())
    }

    pub async fn handle_update_project(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        project_id: Uuid,
        data: UpdateProjectCommand,
        asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let update_req = crate::db::models::project::UpdateProjectRequest {
            name: data.name,
            description: data.description,
            roadmap_id: None, // Not available in websocket command
            target_date: data.target_date.map(Some),
            project_status_id: data.project_status_id,
            priority: data.priority.map(|p| p.parse().unwrap_or_default()),
        };

        let project_info = crate::services::projects_service::ProjectsService::update(
            &mut conn,
            &ctx,
            asset_helper,
            project_id,
            &update_req,
        )?;

        Ok(serde_json::to_value(project_info).unwrap())
    }

    pub async fn handle_delete_project(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        project_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        crate::services::projects_service::ProjectsService::delete(&mut conn, &ctx, project_id)?;
        Ok(serde_json::json!({"deleted": true, "project_id": project_id}))
    }

    pub async fn handle_query_projects(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        filters: ProjectFilters,
        asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let projects = crate::services::projects_service::ProjectsService::list_infos(
            &mut conn,
            &ctx,
            asset_helper,
            filters.search,
            filters.owner_id,
        )?;

        Ok(serde_json::to_value(projects).unwrap())
    }
}
