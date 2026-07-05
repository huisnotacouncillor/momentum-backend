//! Project command handlers

use std::sync::Arc;
use std::str::FromStr;
use uuid::Uuid;
use momentum_core::db::DbPool;
use momentum_core::db::enums::ProjectPriority;
use momentum_core::error::AppError;
use momentum_core::services::context::RequestContext;
use momentum_core::services::projects_service::ProjectsService;
use momentum_core::utils::AssetUrlHelper;
use crate::db::models::project::{CreateProjectRequest, UpdateProjectRequest};
use super::types::*;

pub struct ProjectHandlers;

impl ProjectHandlers {
    pub async fn handle_create_project(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        data: CreateProjectCommand,
        asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        // Convert priority from string to ProjectPriority enum
        let priority = match data.priority {
            Some(p) => Some(ProjectPriority::from_str(&p).map_err(|_| {
                AppError::validation("Invalid priority value")
            })?),
            None => None,
        };

        let req = CreateProjectRequest {
            name: data.name,
            project_key: data.project_key,
            description: data.description,
            target_date: data.target_date,
            project_status_id: data.project_status_id,
            priority,
            roadmap_id: None,
        };

        let project = ProjectsService::create(&mut conn, &ctx, &req)?;
        let project_info = ProjectsService::get_by_id(&mut conn, &ctx, asset_helper, project.id)?;

        Ok(serde_json::json!(project_info))
    }

    pub async fn handle_update_project(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        project_id: Uuid,
        data: UpdateProjectCommand,
        asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        // Convert priority from string to ProjectPriority enum
        let priority = match data.priority {
            Some(p) => Some(ProjectPriority::from_str(&p).map_err(|_| {
                AppError::validation("Invalid priority value")
            })?),
            None => None,
        };

        let req = UpdateProjectRequest {
            name: data.name,
            description: data.description,
            roadmap_id: None,
            target_date: Some(data.target_date),
            project_status_id: data.project_status_id,
            priority,
        };

        let project_info = ProjectsService::update(&mut conn, &ctx, asset_helper, project_id, &req)?;

        Ok(serde_json::json!(project_info))
    }

    pub async fn handle_delete_project(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        project_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        ProjectsService::delete(&mut conn, &ctx, project_id)?;

        Ok(serde_json::json!({
            "project_id": project_id.to_string(),
            "deleted": true
        }))
    }

    pub async fn handle_query_projects(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        filters: ProjectFilters,
        asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let projects = ProjectsService::list_infos(
            &mut conn,
            &ctx,
            asset_helper,
            filters.search,
            filters.owner_id,
        )?;

        Ok(serde_json::json!({
            "items": projects,
            "has_more": false,
            "next_cursor": null
        }))
    }

    pub async fn handle_get_project(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        project_id: Uuid,
        asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let project_info = ProjectsService::get_by_id(&mut conn, &ctx, asset_helper, project_id)?;

        Ok(serde_json::json!(project_info))
    }
}
