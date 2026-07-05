//! User command handlers

use std::sync::Arc;
use uuid::Uuid;
use momentum_core::db::DbPool;
use momentum_core::error::AppError;
use momentum_core::services::context::RequestContext;
use momentum_core::services::auth_service::AuthService;
use momentum_core::utils::AssetUrlHelper;
use super::types::*;

pub struct UserHandlers;

impl UserHandlers {
    pub async fn handle_update_profile(
        _db: &Arc<DbPool>,
        _ctx: RequestContext,
        _data: UpdateProfileCommand,
        _asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("User handlers not yet implemented"))
    }

    pub async fn handle_switch_workspace(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        workspace_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;
        let user = AuthService::switch_workspace(&mut conn, &ctx, workspace_id)?;
        Ok(serde_json::json!(user))
    }
}