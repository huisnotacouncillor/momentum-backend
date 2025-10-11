use diesel::prelude::*;

use crate::{
    db::models::workspace::{NewWorkspace, Workspace},
    db::repositories::workspaces::WorkspacesRepo,
    error::AppError,
};

pub struct WorkspacesService;

impl WorkspacesService {
    pub fn create(
        conn: &mut PgConnection,
        name: &str,
        url_key: &str,
        logo_url: Option<String>,
    ) -> Result<Workspace, AppError> {
        if WorkspacesRepo::exists_url_key(conn, url_key)? {
            return Err(AppError::conflict_with_code(
                "Workspace URL key already exists",
                Some("url_key".into()),
                "WORKSPACE_URL_KEY_EXISTS",
            ));
        }
        let new_ws = NewWorkspace {
            name: name.to_string(),
            url_key: url_key.to_string(),
            logo_url,
        };
        let ws = WorkspacesRepo::insert(conn, &new_ws)?;
        Ok(ws)
    }

    pub fn get_current(
        conn: &mut PgConnection,
        ctx: &crate::services::context::RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
    ) -> Result<crate::db::models::WorkspaceInfo, AppError> {
        let workspace = WorkspacesRepo::find_by_id(conn, ctx.workspace_id)?
            .ok_or_else(|| AppError::not_found("workspace"))?;

        let processed_logo_url = workspace
            .logo_url
            .as_ref()
            .map(|url| asset_helper.process_url(url));

        Ok(crate::db::models::WorkspaceInfo {
            id: workspace.id,
            name: workspace.name,
            url_key: workspace.url_key,
            logo_url: processed_logo_url,
        })
    }

    pub fn update(
        conn: &mut PgConnection,
        ctx: &crate::services::context::RequestContext,
        workspace_id: uuid::Uuid,
        req: &crate::routes::workspaces::UpdateWorkspaceRequest,
    ) -> Result<Workspace, AppError> {
        let existing = WorkspacesRepo::find_by_id(conn, workspace_id)?
            .ok_or_else(|| AppError::not_found("workspace"))?;

        // Verify workspace belongs to user's current workspace (or they have permission)
        if existing.id != ctx.workspace_id {
            return Err(AppError::auth("Cannot update this workspace"));
        }

        let updated = WorkspacesRepo::update_fields(
            conn,
            workspace_id,
            req.name.as_deref(),
            req.url_key.as_deref(),
            req.logo_url.as_deref(),
        )?;
        Ok(updated)
    }

    pub fn delete(
        conn: &mut PgConnection,
        ctx: &crate::services::context::RequestContext,
        workspace_id: uuid::Uuid,
    ) -> Result<(), AppError> {
        let existing = WorkspacesRepo::find_by_id(conn, workspace_id)?
            .ok_or_else(|| AppError::not_found("workspace"))?;

        // Verify workspace belongs to user's current workspace (or they have permission)
        if existing.id != ctx.workspace_id {
            return Err(AppError::auth("Cannot delete this workspace"));
        }

        WorkspacesRepo::delete_by_id(conn, workspace_id)?;
        Ok(())
    }
}
