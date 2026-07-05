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
        db: &Arc<DbPool>,
        ctx: RequestContext,
        data: UpdateProfileCommand,
        asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        // Convert to the format expected by AuthService
        let update_request = momentum_core::services::auth::types::UpdateProfileRequest {
            name: data.name,
            username: data.username,
            email: data.email,
            avatar_url: data.avatar_url,
        };

        let user = AuthService::update_profile(
            &mut conn,
            &ctx,
            &update_request,
            asset_helper,
        )?;
        Ok(serde_json::json!(user))
    }

    pub async fn handle_query_profile(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;
        let user = AuthService::get_profile(&mut conn, &ctx, asset_helper)?;
        Ok(serde_json::json!(user))
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