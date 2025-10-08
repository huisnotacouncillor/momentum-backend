use crate::{error::AppError, services::context::RequestContext, utils::AssetUrlHelper};

use super::types::*;

pub struct UserHandlers;

impl UserHandlers {
    /// Handle profile update via websocket command
    pub async fn handle_update_profile(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        data: UpdateProfileCommand,
        asset_helper: &AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        // Convert websocket command to auth service request format
        let update_request = crate::routes::auth::UpdateProfileRequest {
            name: data.name,
            username: data.username,
            email: data.email,
            avatar_url: data.avatar_url,
        };

        // Use the existing AuthService::update_profile method
        let profile = crate::services::auth_service::AuthService::update_profile(
            &mut conn,
            &ctx,
            &update_request,
            asset_helper,
        )?;

        Ok(serde_json::to_value(profile).unwrap())
    }
}
