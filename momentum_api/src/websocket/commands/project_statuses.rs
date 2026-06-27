//! Project status command handlers stub

use std::sync::Arc;
use uuid::Uuid;
use momentum_core::db::DbPool;
use momentum_core::error::AppError;
use super::types::*;

pub struct ProjectStatusesHandlers;

impl ProjectStatusesHandlers {
    pub async fn handle_get_list(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Project status handlers not yet implemented"))
    }

    pub async fn handle_get_by_id(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _status_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Project status handlers not yet implemented"))
    }

    pub async fn handle_create(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _data: CreateProjectStatusCommand,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Project status handlers not yet implemented"))
    }

    pub async fn handle_update(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _status_id: Uuid,
        _data: UpdateProjectStatusCommand,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Project status handlers not yet implemented"))
    }

    pub async fn handle_delete(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _status_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Project status handlers not yet implemented"))
    }
}