//! Project command handlers stub

use std::sync::Arc;
use uuid::Uuid;
use momentum_core::db::DbPool;
use momentum_core::error::AppError;
use momentum_core::utils::AssetUrlHelper;
use super::types::*;

pub struct ProjectHandlers;

impl ProjectHandlers {
    pub async fn handle_create_project(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _data: CreateProjectCommand,
        _asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Project handlers not yet implemented"))
    }

    pub async fn handle_update_project(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _project_id: Uuid,
        _data: UpdateProjectCommand,
        _asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Project handlers not yet implemented"))
    }

    pub async fn handle_delete_project(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _project_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Project handlers not yet implemented"))
    }

    pub async fn handle_query_projects(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _filters: ProjectFilters,
        _asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Project handlers not yet implemented"))
    }
}