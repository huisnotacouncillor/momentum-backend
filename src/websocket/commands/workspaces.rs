use uuid::Uuid;

use crate::{error::AppError, services::context::RequestContext};

use super::types::*;

pub struct WorkspaceHandlers;

impl WorkspaceHandlers {
    pub async fn handle_create_workspace(
        db: &crate::db::DbPool,
        _ctx: RequestContext,
        data: CreateWorkspaceCommand,
        asset_helper: &crate::utils::AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        if data.name.trim().is_empty() {
            return Err(AppError::validation("Workspace name is required"));
        }
        if data.url_key.trim().is_empty() {
            return Err(AppError::validation("Workspace URL key is required"));
        }
        if !data
            .url_key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::validation(
                "Workspace URL key can only contain letters, numbers, hyphens, and underscores",
            ));
        }
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let workspace = crate::services::workspaces_service::WorkspacesService::create(
            &mut conn,
            &data.name,
            &data.url_key,
            data.logo_url,
        )?;

        // Process logo_url with asset_helper
        let processed_logo_url = workspace
            .logo_url
            .as_ref()
            .map(|url| asset_helper.process_url(url));

        Ok(serde_json::json!({
            "id": workspace.id,
            "name": workspace.name,
            "url_key": workspace.url_key,
            "logo_url": processed_logo_url,
            "created_at": workspace.created_at,
        }))
    }

    pub async fn handle_update_workspace(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        workspace_id: Uuid,
        data: UpdateWorkspaceCommand,
        asset_helper: &crate::utils::AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let req = crate::routes::workspaces::UpdateWorkspaceRequest {
            name: data.name,
            url_key: data.url_key,
            logo_url: data.logo_url,
        };
        let workspace = crate::services::workspaces_service::WorkspacesService::update(
            &mut conn,
            &ctx,
            workspace_id,
            &req,
        )?;

        // Process logo_url with asset_helper
        let processed_logo_url = workspace
            .logo_url
            .as_ref()
            .map(|url| asset_helper.process_url(url));

        Ok(serde_json::json!({
            "id": workspace.id,
            "name": workspace.name,
            "url_key": workspace.url_key,
            "logo_url": processed_logo_url,
            "created_at": workspace.created_at,
        }))
    }

    pub async fn handle_delete_workspace(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        workspace_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        crate::services::workspaces_service::WorkspacesService::delete(
            &mut conn,
            &ctx,
            workspace_id,
        )?;
        Ok(serde_json::json!({"deleted": true, "workspace_id": workspace_id}))
    }

    pub async fn handle_get_current_workspace(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let workspace = crate::services::workspaces_service::WorkspacesService::get_current(
            &mut conn,
            &ctx,
            asset_helper,
        )?;
        Ok(serde_json::to_value(workspace).unwrap())
    }
}
