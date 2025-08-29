use crate::db::{DbPool, models::*};
use crate::middleware::auth::AuthUserInfo;
use crate::schema;
use axum::{
    Json,
    extract::{Query, State},
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

    let user_id = auth_info.user.id;
    let _current_workspace_id = auth_info.current_workspace_id.unwrap();

    // 检查 project_key 格式
    if !payload.project_key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("project_key".to_string()),
            code: "INVALID_FORMAT".to_string(),
            message: "Project key can only contain letters, numbers, hyphens, and underscores".to_string(),
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

    // 创建项目
    let new_project = NewProject {
        name: payload.name.clone(),
        project_key: payload.project_key.clone(),
        description: payload.description.clone(),
        workspace_id: _current_workspace_id,
        owner_id: user_id, // 使用当前用户作为项目所有者
        roadmap_id: None, // 暂时设置为None，可以根据需要修改
        target_date: None, // 暂时设置为None，可以根据需要修改
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

    let _response = ApiResponse::success(
        Some(project.clone()),
        "Project created successfully",
    );

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

    let query = projects
        .filter(workspace_id.eq(workspace_id))
        .select(Project::as_select())
        .order(created_at.desc());

    let results = if let Some(search_term) = params.search {
        let pattern = format!("%{}%", search_term.to_lowercase());
        query
            .filter(name.ilike(&pattern))
            .load::<Project>(&mut conn)
    } else {
        query.load::<Project>(&mut conn)
    };

    let projects_list = match results {
        Ok(list) => list,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve projects");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(projects_list),
        "Projects retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}