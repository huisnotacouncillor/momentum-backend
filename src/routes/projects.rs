use crate::db::enums::ProjectPriority;
use crate::db::models::api::{ApiResponse, ErrorDetail};
use crate::db::{DbPool, models::*};
use crate::middleware::auth::AuthUserInfo;
use crate::schema;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
// 移除未使用的 Uuid 导入

#[derive(Deserialize)]
pub struct ProjectQueryParams {
    pub search: Option<String>,
    pub owner_id: Option<uuid::Uuid>,
}

pub async fn create_project(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let _workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    // 验证项目名称不为空
    if payload.name.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("name".to_string()),
            code: "REQUIRED".to_string(),
            message: "Project name is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 验证项目键不为空且格式正确
    if payload.project_key.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("project_key".to_string()),
            code: "REQUIRED".to_string(),
            message: "Project key is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 验证项目键长度（最大10个字符）
    if payload.project_key.len() > 10 {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("project_key".to_string()),
            code: "INVALID_LENGTH".to_string(),
            message: "Project key must be 10 characters or less".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 获取用户的当前工作空间
    let user = match schema::users::table
        .filter(schema::users::id.eq(auth_info.user.id))
        .filter(schema::users::is_active.eq(true))
        .select(User::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            let response = ApiResponse::<()>::unauthorized("User not found or inactive");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let _workspace_id = match user.current_workspace_id {
        Some(id) => id,
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    let _user_id = auth_info.user.id;
    let _current_workspace_id = auth_info.current_workspace_id.unwrap();

    // 检查 project_key 格式
    if !payload
        .project_key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("project_key".to_string()),
            code: "INVALID_FORMAT".to_string(),
            message: "Project key can only contain letters, numbers, hyphens, and underscores"
                .to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }
    let key_exists = match schema::projects::table
        .filter(schema::projects::workspace_id.eq(_current_workspace_id))
        .filter(schema::projects::project_key.eq(&payload.project_key))
        .select(schema::projects::id)
        .first::<uuid::Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if key_exists.is_some() {
        let response = ApiResponse::<()>::conflict(
            "Project key already exists in this workspace",
            Some("project_key".to_string()),
            "PROJECT_KEY_EXISTS",
        );
        return (StatusCode::CONFLICT, Json(response)).into_response();
    }

    // 获取默认项目状态
    let default_status = match crate::schema::project_statuses::table
        .filter(crate::schema::project_statuses::workspace_id.eq(_workspace_id))
        .filter(crate::schema::project_statuses::name.eq("Planned"))
        .first::<crate::db::models::project_status::ProjectStatus>(&mut conn)
    {
        Ok(status) => status.id,
        Err(_) => {
            // 如果找不到"Planned"状态，则使用第一个可用的状态
            match crate::schema::project_statuses::table
                .filter(crate::schema::project_statuses::workspace_id.eq(_workspace_id))
                .first::<crate::db::models::project_status::ProjectStatus>(&mut conn)
            {
                Ok(status) => status.id,
                Err(_) => {
                    let response =
                        ApiResponse::<()>::internal_error("No project statuses available");
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                }
            }
        }
    };

    let new_project = NewProject {
        workspace_id: _workspace_id,
        roadmap_id: payload.roadmap_id,
        owner_id: auth_info.user.id,
        name: payload.name.clone(),
        project_key: payload.project_key.clone(),
        description: payload.description.clone(),
        project_status_id: default_status,
        target_date: payload.target_date,
        priority: Some(payload.priority.unwrap_or(ProjectPriority::None)),
    };

    let project = match diesel::insert_into(schema::projects::table)
        .values(&new_project)
        .get_result::<Project>(&mut conn)
    {
        Ok(project) => project,
        Err(e) => {
            // 检查是否是唯一性约束违反错误
            if e.to_string().contains("project_key") {
                let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                    field: Some("project_key".to_string()),
                    code: "DUPLICATE".to_string(),
                    message: "Project with this key already exists in this workspace".to_string(),
                }]);
                return (StatusCode::BAD_REQUEST, Json(response)).into_response();
            } else {
                let response = ApiResponse::<()>::internal_error("Failed to create project");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    };

    let _response = ApiResponse::success(Some(project.clone()), "Project created successfully");

    let response = ApiResponse::created(project, "Project created successfully");
    (StatusCode::CREATED, Json(response)).into_response()
}

pub async fn get_projects(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Query(params): Query<ProjectQueryParams>,
) -> impl IntoResponse {
    let _current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    use crate::schema::projects::dsl::*;

    let mut query = projects
        .filter(workspace_id.eq(_current_workspace_id))
        .into_boxed();

    if let Some(oid) = params.owner_id {
        query = query.filter(owner_id.eq(oid));
    }

    // 排序在最后统一应用
    let results = if let Some(search_term) = params.search {
        let pattern = format!("%{}%", search_term.to_lowercase());
        query
            .filter(name.ilike(&pattern))
            .order(created_at.desc())
            .load::<Project>(&mut conn)
    } else {
        query.order(created_at.desc()).load::<Project>(&mut conn)
    };

    let projects_list = match results {
        Ok(list) => list,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve projects");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 获取项目状态信息
    let mut projects_with_status = Vec::new();
    for project in projects_list {
        let project_status = match crate::schema::project_statuses::table
            .filter(crate::schema::project_statuses::id.eq(project.project_status_id))
            .select(crate::db::models::project_status::ProjectStatus::as_select())
            .first::<crate::db::models::project_status::ProjectStatus>(&mut conn)
            .optional()
        {
            Ok(Some(status)) => status,
            Ok(None) => {
                let response = ApiResponse::<()>::internal_error("Project status not found");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
            Err(_) => {
                let response =
                    ApiResponse::<()>::internal_error("Failed to retrieve project status");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        };

        let project_status_info = crate::db::models::project_status::ProjectStatusInfo {
            id: project_status.id,
            name: project_status.name,
            description: project_status.description,
            color: project_status.color,
            category: project_status.category,
            created_at: project_status.created_at,
            updated_at: project_status.updated_at,
        };

        // 获取项目所有者信息
        let owner = match schema::users::table
            .filter(schema::users::id.eq(project.owner_id))
            .select(User::as_select())
            .first::<User>(&mut conn)
            .optional()
        {
            Ok(Some(user)) => user,
            _ => {
                let response =
                    ApiResponse::<()>::internal_error("Failed to retrieve project owner");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        };
        let owner_basic = crate::db::models::auth::UserBasicInfo {
            id: owner.id,
            name: owner.name,
            username: owner.username,
            email: owner.email,
            avatar_url: owner.avatar_url,
        };

        let project_info = crate::db::models::project::ProjectInfo {
            id: project.id,
            name: project.name,
            project_key: project.project_key,
            description: project.description,
            status: project_status_info,
            owner: owner_basic,
            target_date: project.target_date,
            created_at: project.created_at,
            updated_at: project.updated_at,
            priority: project.priority,
        };

        projects_with_status.push(project_info);
    }

    let response = ApiResponse::success(
        Some(projects_with_status),
        "Projects retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn update_project(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(project_id): Path<uuid::Uuid>,
    Json(payload): Json<UpdateProjectRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let current_workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    // 首先检查项目是否存在且属于当前工作区
    let existing_project = match schema::projects::table
        .filter(schema::projects::id.eq(project_id))
        .filter(schema::projects::workspace_id.eq(current_workspace_id))
        .select(Project::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(Some(project)) => project,
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Project not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查是否提供了更新字段
    if payload.name.is_none()
        && payload.description.is_none()
        && payload.roadmap_id.is_none()
        && payload.target_date.is_none()
        && payload.project_status_id.is_none()
        && payload.priority.is_none()
    {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: None,
            code: "NO_UPDATE_DATA".to_string(),
            message: "No update data provided".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 构建更新查询
    let query = schema::projects::table
        .filter(schema::projects::id.eq(project_id))
        .filter(schema::projects::workspace_id.eq(current_workspace_id));

    // 根据提供的字段执行不同的更新操作
    let updated_project = if let Some(name) = payload.name {
        if name.trim().is_empty() {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: Some("name".to_string()),
                code: "REQUIRED".to_string(),
                message: "Project name cannot be empty".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }

        match diesel::update(query.clone())
            .set((
                schema::projects::name.eq(name),
                schema::projects::updated_at.eq(chrono::Utc::now()),
            ))
            .get_result::<Project>(&mut conn)
        {
            Ok(project) => project,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Failed to update project");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    } else if let Some(description) = payload.description {
        match diesel::update(query.clone())
            .set((
                schema::projects::description.eq(description),
                schema::projects::updated_at.eq(chrono::Utc::now()),
            ))
            .get_result::<Project>(&mut conn)
        {
            Ok(project) => project,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Failed to update project");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    } else if let Some(roadmap_id) = payload.roadmap_id {
        match diesel::update(query.clone())
            .set((
                schema::projects::roadmap_id.eq(roadmap_id),
                schema::projects::updated_at.eq(chrono::Utc::now()),
            ))
            .get_result::<Project>(&mut conn)
        {
            Ok(project) => project,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Failed to update project");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    } else if let Some(target_date) = payload.target_date {
        match diesel::update(query.clone())
            .set((
                schema::projects::target_date.eq(target_date),
                schema::projects::updated_at.eq(chrono::Utc::now()),
            ))
            .get_result::<Project>(&mut conn)
        {
            Ok(project) => project,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Failed to update project");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    } else if let Some(project_status_id) = payload.project_status_id {
        // 验证项目状态是否存在且属于当前工作区
        let status_exists = match schema::project_statuses::table
            .filter(schema::project_statuses::id.eq(project_status_id))
            .filter(schema::project_statuses::workspace_id.eq(current_workspace_id))
            .select(schema::project_statuses::id)
            .first::<uuid::Uuid>(&mut conn)
            .optional()
        {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Database error");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        };

        if !status_exists {
            let response =
                ApiResponse::<()>::not_found("Project status not found in current workspace");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }

        match diesel::update(query.clone())
            .set((
                schema::projects::project_status_id.eq(project_status_id),
                schema::projects::updated_at.eq(chrono::Utc::now()),
            ))
            .get_result::<Project>(&mut conn)
        {
            Ok(project) => project,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Failed to update project");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    } else if let Some(priority) = payload.priority {
        match diesel::update(query.clone())
            .set((
                schema::projects::priority.eq(priority),
                schema::projects::updated_at.eq(chrono::Utc::now()),
            ))
            .get_result::<Project>(&mut conn)
        {
            Ok(project) => project,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Failed to update project");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    } else {
        // 如果没有提供任何更新字段，返回原始项目
        existing_project
    };

    // 获取项目状态信息
    let project_status = match crate::schema::project_statuses::table
        .filter(crate::schema::project_statuses::id.eq(updated_project.project_status_id))
        .select(crate::db::models::project_status::ProjectStatus::as_select())
        .first::<crate::db::models::project_status::ProjectStatus>(&mut conn)
        .optional()
    {
        Ok(Some(status)) => status,
        Ok(None) => {
            let response = ApiResponse::<()>::internal_error("Project status not found");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve project status");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let project_status_info = crate::db::models::project_status::ProjectStatusInfo {
        id: project_status.id,
        name: project_status.name,
        description: project_status.description,
        color: project_status.color,
        category: project_status.category,
        created_at: project_status.created_at,
        updated_at: project_status.updated_at,
    };

    // 获取项目所有者信息
    let owner = match schema::users::table
        .filter(schema::users::id.eq(updated_project.owner_id))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .optional()
    {
        Ok(Some(user)) => user,
        _ => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve project owner");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    let owner_basic = crate::db::models::auth::UserBasicInfo {
        id: owner.id,
        name: owner.name,
        username: owner.username,
        email: owner.email,
        avatar_url: owner.avatar_url,
    };

    let project_info = crate::db::models::project::ProjectInfo {
        id: updated_project.id,
        name: updated_project.name,
        project_key: updated_project.project_key,
        description: updated_project.description,
        status: project_status_info,
        owner: owner_basic,
        target_date: updated_project.target_date,
        created_at: updated_project.created_at,
        updated_at: updated_project.updated_at,
        priority: updated_project.priority,
    };

    let response = ApiResponse::success(Some(project_info), "Project updated successfully");
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn delete_project(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(project_id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let current_workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    // 检查项目是否存在且属于当前工作区
    let _existing_project = match schema::projects::table
        .filter(schema::projects::id.eq(project_id))
        .filter(schema::projects::workspace_id.eq(current_workspace_id))
        .select(schema::projects::id)
        .first::<uuid::Uuid>(&mut conn)
        .optional()
    {
        Ok(Some(project)) => project,
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Project not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 执行删除
    match diesel::delete(
        schema::projects::table
            .filter(schema::projects::id.eq(project_id))
            .filter(schema::projects::workspace_id.eq(current_workspace_id)),
    )
    .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::success(Option::<()>::None, "Project deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to delete project");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}
