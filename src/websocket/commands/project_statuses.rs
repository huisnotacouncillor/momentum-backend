use uuid::Uuid;

use crate::{error::AppError, services::context::RequestContext};

use super::types::*;

pub struct ProjectStatusesHandlers;

impl ProjectStatusesHandlers {
    pub async fn handle_get_list(
        db: &crate::db::DbPool,
        ctx: RequestContext,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let list = crate::services::project_statuses_service::ProjectStatusesService::list(
            &mut conn, &ctx,
        )?;
        Ok(serde_json::to_value(list).unwrap())
    }

    pub async fn handle_get_by_id(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        status_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let item = crate::services::project_statuses_service::ProjectStatusesService::get_by_id(
            &mut conn, &ctx, status_id,
        )?;
        Ok(serde_json::to_value(item).unwrap())
    }

    pub async fn handle_create(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        data: CreateProjectStatusCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let category_enum = match data.category.as_str() {
            "backlog" => crate::db::models::project_status::ProjectStatusCategory::Backlog,
            "planned" => crate::db::models::project_status::ProjectStatusCategory::Planned,
            "in_progress" => crate::db::models::project_status::ProjectStatusCategory::InProgress,
            "completed" => crate::db::models::project_status::ProjectStatusCategory::Completed,
            "canceled" => crate::db::models::project_status::ProjectStatusCategory::Canceled,
            _ => crate::db::models::project_status::ProjectStatusCategory::Backlog,
        };
        let model_request = crate::db::models::project_status::CreateProjectStatusRequest {
            name: data.name,
            description: data.description,
            color: Some(data.color),
            category: category_enum,
        };
        let created = crate::services::project_statuses_service::ProjectStatusesService::create(
            &mut conn,
            &ctx,
            &model_request,
        )?;
        Ok(serde_json::to_value(created).unwrap())
    }

    pub async fn handle_update(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        status_id: Uuid,
        data: UpdateProjectStatusCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let req = crate::routes::project_statuses::UpdateProjectStatusRequest {
            name: data.name,
            description: data.description,
            color: data.color,
            category: data.category,
        };
        let updated = crate::services::project_statuses_service::ProjectStatusesService::update(
            &mut conn, &ctx, status_id, &req,
        )?;
        Ok(serde_json::to_value(updated).unwrap())
    }

    pub async fn handle_delete(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        status_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        crate::services::project_statuses_service::ProjectStatusesService::delete(
            &mut conn, &ctx, status_id,
        )?;
        Ok(serde_json::json!({"deleted": true, "status_id": status_id}))
    }
}
