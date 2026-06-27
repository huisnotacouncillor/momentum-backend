//! User command handlers stub

use std::sync::Arc;
use momentum_core::db::DbPool;
use momentum_core::error::AppError;
use momentum_core::utils::AssetUrlHelper;
use super::types::*;

pub struct UserHandlers;

impl UserHandlers {
    pub async fn handle_update_profile(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _data: UpdateProfileCommand,
        _asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("User handlers not yet implemented"))
    }
}