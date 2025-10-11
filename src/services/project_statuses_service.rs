use diesel::prelude::*;

use crate::{
    db::models::project_status::{NewProjectStatus, ProjectStatusInfo},
    db::repositories::project_statuses::ProjectStatusRepo,
    error::AppError,
    services::context::RequestContext,
    validation::project_status::{
        UpdateProjectStatusChanges, validate_create_project_status, validate_update_project_status,
    },
};

pub struct ProjectStatusesService;

impl ProjectStatusesService {
    pub fn list(
        conn: &mut PgConnection,
        ctx: &RequestContext,
    ) -> Result<Vec<ProjectStatusInfo>, AppError> {
        let results = ProjectStatusRepo::list_by_workspace(conn, ctx.workspace_id)?;
        let list = results
            .into_iter()
            .map(|s| ProjectStatusInfo {
                id: s.id,
                name: s.name,
                description: s.description,
                color: s.color,
                category: s.category,
                created_at: s.created_at,
                updated_at: s.updated_at,
            })
            .collect();
        Ok(list)
    }

    pub fn create(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        req: &crate::db::models::project_status::CreateProjectStatusRequest,
    ) -> Result<ProjectStatusInfo, AppError> {
        validate_create_project_status(&req.name, &req.color)?;
        if ProjectStatusRepo::exists_by_name(conn, ctx.workspace_id, &req.name)? {
            return Err(AppError::conflict_with_code(
                "Project status already exists",
                Some("name".into()),
                "PROJECT_STATUS_EXISTS",
            ));
        }
        let new_status = NewProjectStatus {
            name: req.name.clone(),
            description: req.description.clone(),
            color: req.color.clone(),
            category: req.category,
            workspace_id: ctx.workspace_id,
        };
        let created = ProjectStatusRepo::insert(conn, &new_status)?;
        Ok(ProjectStatusInfo {
            id: created.id,
            name: created.name,
            description: created.description,
            color: created.color,
            category: created.category,
            created_at: created.created_at,
            updated_at: created.updated_at,
        })
    }

    pub fn get_by_id(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        status_id: uuid::Uuid,
    ) -> Result<ProjectStatusInfo, AppError> {
        let status = ProjectStatusRepo::find_by_id_in_workspace(conn, ctx.workspace_id, status_id)?
            .ok_or_else(|| AppError::not_found("project_status"))?;

        Ok(ProjectStatusInfo {
            id: status.id,
            name: status.name,
            description: status.description,
            color: status.color,
            category: status.category,
            created_at: status.created_at,
            updated_at: status.updated_at,
        })
    }

    pub fn update(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        status_id: uuid::Uuid,
        req: &crate::routes::project_statuses::UpdateProjectStatusRequest,
    ) -> Result<ProjectStatusInfo, AppError> {
        let existing =
            ProjectStatusRepo::find_by_id_in_workspace(conn, ctx.workspace_id, status_id)?;
        let Some(_row) = existing else {
            return Err(AppError::not_found("project_status"));
        };
        let changes = UpdateProjectStatusChanges {
            name: req.name.as_deref(),
            description_present: req.description.is_some(),
            color: req.color.as_deref(),
            category: req.category.as_ref().and_then(|cat| match cat.as_str() {
                "backlog" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::Backlog)
                }
                "planned" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::Planned)
                }
                "in_progress" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::InProgress)
                }
                "completed" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::Completed)
                }
                "canceled" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::Canceled)
                }
                _ => None,
            }),
        };
        validate_update_project_status(&changes)?;
        if let Some(name) = &req.name {
            if ProjectStatusRepo::exists_by_name_excluding_id(
                conn,
                ctx.workspace_id,
                name,
                status_id,
            )? {
                return Err(AppError::conflict_with_code(
                    "Project status already exists",
                    Some("name".into()),
                    "PROJECT_STATUS_EXISTS",
                ));
            }
        }
        let updated = ProjectStatusRepo::update_fields(
            conn,
            status_id,
            req.name.clone(),
            Some(req.description.clone()),
            Some(req.color.clone()),
            req.category.as_ref().and_then(|cat| match cat.as_str() {
                "backlog" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::Backlog)
                }
                "planned" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::Planned)
                }
                "in_progress" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::InProgress)
                }
                "completed" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::Completed)
                }
                "canceled" => {
                    Some(crate::db::models::project_status::ProjectStatusCategory::Canceled)
                }
                _ => None,
            }),
        )?;
        Ok(ProjectStatusInfo {
            id: updated.id,
            name: updated.name,
            description: updated.description,
            color: updated.color,
            category: updated.category,
            created_at: updated.created_at,
            updated_at: updated.updated_at,
        })
    }

    pub fn delete(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        status_id: uuid::Uuid,
    ) -> Result<(), AppError> {
        let existing =
            ProjectStatusRepo::find_by_id_in_workspace(conn, ctx.workspace_id, status_id)?;
        if existing.is_none() {
            return Err(AppError::not_found("project_status"));
        }
        ProjectStatusRepo::delete_by_id(conn, status_id)?;
        Ok(())
    }
}
