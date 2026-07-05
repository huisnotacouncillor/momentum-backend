//! Project status command handlers

use std::sync::Arc;
use uuid::Uuid;
use momentum_core::db::DbPool;
use momentum_core::db::models::project_status::{CreateProjectStatusRequest as DbCreateRequest, ProjectStatusCategory};
use momentum_core::error::AppError;
use momentum_core::services::context::RequestContext;
use momentum_core::services::project_statuses_service::ProjectStatusesService;
use momentum_core::services::project_statuses::types::UpdateProjectStatusRequest as DbUpdateRequest;
use super::types::*;

pub struct ProjectStatusesHandlers;

impl ProjectStatusesHandlers {
    pub async fn handle_get_list(
        db: &Arc<DbPool>,
        ctx: RequestContext,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let statuses = ProjectStatusesService::list(&mut conn, &ctx)?;

        Ok(serde_json::json!({
            "items": statuses,
            "has_more": false,
            "next_cursor": null
        }))
    }

    pub async fn handle_get_by_id(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        status_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let status = ProjectStatusesService::get_by_id(&mut conn, &ctx, status_id)?;

        Ok(serde_json::json!(status))
    }

    pub async fn handle_create(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        data: CreateProjectStatusCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let category = match data.category.as_str() {
            "backlog" => ProjectStatusCategory::Backlog,
            "planned" => ProjectStatusCategory::Planned,
            "in_progress" => ProjectStatusCategory::InProgress,
            "completed" => ProjectStatusCategory::Completed,
            "canceled" => ProjectStatusCategory::Canceled,
            _ => ProjectStatusCategory::Backlog,
        };

        let req = DbCreateRequest {
            name: data.name,
            description: data.description,
            color: Some(data.color),
            category,
        };

        let status = ProjectStatusesService::create(&mut conn, &ctx, &req)?;

        Ok(serde_json::json!(status))
    }

    pub async fn handle_update(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        status_id: Uuid,
        data: UpdateProjectStatusCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let req = DbUpdateRequest {
            name: data.name,
            description: data.description,
            color: data.color,
            category: data.category,
        };

        let status = ProjectStatusesService::update(&mut conn, &ctx, status_id, &req)?;

        Ok(serde_json::json!(status))
    }

    pub async fn handle_delete(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        status_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        ProjectStatusesService::delete(&mut conn, &ctx, status_id)?;

        Ok(serde_json::json!({
            "status_id": status_id.to_string(),
            "deleted": true
        }))
    }
}
