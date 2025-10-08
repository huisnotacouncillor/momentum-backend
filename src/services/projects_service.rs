use diesel::prelude::*;

use crate::{
    db::models::project::{NewProject, Project, ProjectInfo},
    db::repositories::projects::ProjectsRepo,
    error::AppError,
    services::context::RequestContext,
    validation::project::validate_create_project,
};

pub struct ProjectsService;

impl ProjectsService {
    pub fn create(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        req: &crate::db::models::project::CreateProjectRequest,
    ) -> Result<Project, AppError> {
        validate_create_project(&req.name, &req.project_key)?;
        if ProjectsRepo::exists_key_in_workspace(conn, ctx.workspace_id, &req.project_key)? {
            return Err(AppError::conflict_with_code(
                "Project key already exists in this workspace",
                Some("project_key".into()),
                "PROJECT_KEY_EXISTS",
            ));
        }

        // Resolve default status similar to routes/projects.rs logic
        let default_status = match crate::schema::project_statuses::table
            .filter(crate::schema::project_statuses::workspace_id.eq(ctx.workspace_id))
            .filter(crate::schema::project_statuses::name.eq("Planned"))
            .select(crate::schema::project_statuses::id)
            .first::<uuid::Uuid>(conn)
            .optional()
        {
            Ok(Some(id)) => id,
            _ => {
                match crate::schema::project_statuses::table
                    .filter(crate::schema::project_statuses::workspace_id.eq(ctx.workspace_id))
                    .select(crate::schema::project_statuses::id)
                    .first::<uuid::Uuid>(conn)
                    .optional()
                {
                    Ok(Some(id)) => id,
                    _ => return Err(AppError::internal("No project statuses available")),
                }
            }
        };

        let new_project = NewProject {
            workspace_id: ctx.workspace_id,
            roadmap_id: req.roadmap_id,
            owner_id: ctx.user_id,
            name: req.name.clone(),
            project_key: req.project_key.clone(),
            description: req.description.clone(),
            project_status_id: req.project_status_id.unwrap_or(default_status),
            target_date: req.target_date,
            priority: req.priority.clone(),
        };

        let created = ProjectsRepo::insert(conn, &new_project)?;
        Ok(created)
    }

    pub fn list_infos(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
        search: Option<String>,
        owner_id_filter: Option<uuid::Uuid>,
    ) -> Result<Vec<ProjectInfo>, AppError> {
        use crate::schema::projects::dsl as p;
        let mut query = p::projects
            .filter(p::workspace_id.eq(ctx.workspace_id))
            .into_boxed();
        if let Some(owner) = owner_id_filter {
            query = query.filter(p::owner_id.eq(owner));
        }
        if let Some(search_term) = search {
            let pattern = format!("%{}%", search_term.to_lowercase());
            query = query.filter(p::name.ilike(pattern));
        }
        let list = query.order(p::created_at.desc()).load::<Project>(conn)?;

        // Get all project statuses for this workspace once
        let all_statuses = crate::schema::project_statuses::table
            .filter(crate::schema::project_statuses::workspace_id.eq(ctx.workspace_id))
            .select(crate::db::models::project_status::ProjectStatus::as_select())
            .load::<crate::db::models::project_status::ProjectStatus>(conn)?;

        let available_statuses: Vec<crate::db::models::project_status::ProjectStatusInfo> =
            all_statuses
                .into_iter()
                .map(
                    |status| crate::db::models::project_status::ProjectStatusInfo {
                        id: status.id,
                        name: status.name,
                        description: status.description,
                        color: status.color,
                        category: status.category,
                        created_at: status.created_at,
                        updated_at: status.updated_at,
                    },
                )
                .collect();

        // Assemble infos
        let mut infos = Vec::with_capacity(list.len());
        for project in list {
            // status
            let status = crate::schema::project_statuses::table
                .filter(crate::schema::project_statuses::id.eq(project.project_status_id))
                .select(crate::db::models::project_status::ProjectStatus::as_select())
                .first::<crate::db::models::project_status::ProjectStatus>(conn)
                .optional()?;
            let status = status.ok_or_else(|| AppError::internal("Project status not found"))?;
            let status_info = crate::db::models::project_status::ProjectStatusInfo {
                id: status.id,
                name: status.name,
                description: status.description,
                color: status.color,
                category: status.category,
                created_at: status.created_at,
                updated_at: status.updated_at,
            };
            // owner
            let owner = crate::schema::users::table
                .filter(crate::schema::users::id.eq(project.owner_id))
                .select(crate::db::models::auth::User::as_select())
                .first::<crate::db::models::auth::User>(conn)
                .optional()?;
            let owner =
                owner.ok_or_else(|| AppError::internal("Failed to retrieve project owner"))?;
            let processed_avatar_url = owner
                .avatar_url
                .as_ref()
                .map(|url| asset_helper.process_url(url));
            let owner_basic = crate::db::models::auth::UserBasicInfo {
                id: owner.id,
                name: owner.name,
                username: owner.username,
                email: owner.email,
                avatar_url: processed_avatar_url,
            };

            infos.push(ProjectInfo {
                id: project.id,
                name: project.name,
                project_key: project.project_key,
                description: project.description,
                status: status_info,
                available_statuses: available_statuses.clone(), // 为每个项目添加所有可用的状态选项
                owner: owner_basic,
                target_date: project.target_date,
                priority: project.priority,
                created_at: project.created_at,
                updated_at: project.updated_at,
            });
        }
        Ok(infos)
    }

    pub fn update(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        asset_helper: &crate::utils::AssetUrlHelper,
        project_id: uuid::Uuid,
        req: &crate::db::models::project::UpdateProjectRequest,
    ) -> Result<crate::db::models::project::ProjectInfo, AppError> {
        // Check if project exists and belongs to workspace
        let existing = ProjectsRepo::find_by_id_in_workspace(conn, ctx.workspace_id, project_id)?;
        let Some(_project) = existing else {
            return Err(AppError::not_found("project"));
        };

        // Update fields
        let updated = ProjectsRepo::update_fields(
            conn,
            project_id,
            req.name.as_ref().map(|s| s.as_str()),
            req.description.as_ref().map(|s| s.as_str()),
            None, // project_key not available in update request
            req.project_status_id,
            req.target_date.map(|opt_date| {
                opt_date.map(|d| {
                    chrono::NaiveDateTime::from(d.and_hms_opt(0, 0, 0).unwrap_or_default())
                })
            }),
            req.priority.as_ref(),
            req.roadmap_id,
        )?;

        // Build ProjectInfo response
        let status = crate::schema::project_statuses::table
            .filter(crate::schema::project_statuses::id.eq(updated.project_status_id))
            .select(crate::db::models::project_status::ProjectStatus::as_select())
            .first::<crate::db::models::project_status::ProjectStatus>(conn)
            .optional()?
            .ok_or_else(|| AppError::internal("Project status not found"))?;

        let owner = crate::schema::users::table
            .filter(crate::schema::users::id.eq(updated.owner_id))
            .select(crate::db::models::auth::User::as_select())
            .first::<crate::db::models::auth::User>(conn)
            .optional()?
            .ok_or_else(|| AppError::internal("Failed to retrieve project owner"))?;

        let processed_avatar_url = owner
            .avatar_url
            .as_ref()
            .map(|url| asset_helper.process_url(url));
        let owner_basic = crate::db::models::auth::UserBasicInfo {
            id: owner.id,
            name: owner.name,
            username: owner.username,
            email: owner.email,
            avatar_url: processed_avatar_url,
        };

        let status_info = crate::db::models::project_status::ProjectStatusInfo {
            id: status.id,
            name: status.name,
            description: status.description,
            color: status.color,
            category: status.category,
            created_at: status.created_at,
            updated_at: status.updated_at,
        };

        // Get all available statuses for this workspace
        let all_statuses = crate::schema::project_statuses::table
            .filter(crate::schema::project_statuses::workspace_id.eq(ctx.workspace_id))
            .select(crate::db::models::project_status::ProjectStatus::as_select())
            .load::<crate::db::models::project_status::ProjectStatus>(conn)?;

        let available_statuses: Vec<crate::db::models::project_status::ProjectStatusInfo> =
            all_statuses
                .into_iter()
                .map(
                    |status| crate::db::models::project_status::ProjectStatusInfo {
                        id: status.id,
                        name: status.name,
                        description: status.description,
                        color: status.color,
                        category: status.category,
                        created_at: status.created_at,
                        updated_at: status.updated_at,
                    },
                )
                .collect();

        Ok(crate::db::models::project::ProjectInfo {
            id: updated.id,
            name: updated.name,
            project_key: updated.project_key,
            description: updated.description,
            status: status_info,
            available_statuses,
            owner: owner_basic,
            target_date: updated.target_date,
            priority: updated.priority,
            created_at: updated.created_at,
            updated_at: updated.updated_at,
        })
    }

    pub fn delete(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        project_id: uuid::Uuid,
    ) -> Result<(), AppError> {
        // Check if project exists and belongs to workspace
        let existing = ProjectsRepo::find_by_id_in_workspace(conn, ctx.workspace_id, project_id)?;
        if existing.is_none() {
            return Err(AppError::not_found("project"));
        }

        ProjectsRepo::delete_by_id(conn, project_id)?;
        Ok(())
    }
}
