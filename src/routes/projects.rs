use crate::db::{DbPool, models::*};
use crate::middleware::auth::{AuthConfig, AuthService};
use crate::schema;
use axum::TypedHeader;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use headers::Authorization;
use headers::authorization::Bearer;
use std::sync::Arc;

pub async fn create_project(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
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
        .filter(schema::users::id.eq(claims.sub))
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

    let workspace_id = match user.current_workspace_id {
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

    // 验证用户对工作空间的访问权限
    let user_has_access = match schema::team_members::table
        .inner_join(schema::teams::table.on(schema::teams::id.eq(schema::team_members::team_id)))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .filter(schema::teams::workspace_id.eq(workspace_id))
        .select(schema::team_members::user_id)
        .first::<uuid::Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if user_has_access.is_none() {
        let response = ApiResponse::<()>::forbidden("You don't have access to this workspace");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 检查项目键在工作空间中是否唯一
    let key_exists = match schema::projects::table
        .filter(schema::projects::workspace_id.eq(workspace_id))
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

    // 创建新项目
    let new_project = NewProject {
        workspace_id,
        roadmap_id: payload.roadmap_id,
        owner_id: claims.sub,
        name: payload.name.trim().to_string(),
        project_key: payload.project_key.trim().to_uppercase(),
        description: payload
            .description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty()),
        target_date: payload.target_date,
    };

    let project = match diesel::insert_into(schema::projects::table)
        .values(&new_project)
        .get_result::<Project>(&mut conn)
    {
        Ok(project) => project,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to create project");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 构建项目信息响应
    let owner_info = UserBasicInfo {
        id: user.id,
        name: user.name,
        username: user.username,
        email: user.email,
        avatar_url: user.avatar_url,
    };

    let project_info = ProjectInfo {
        id: project.id,
        name: project.name,
        project_key: project.project_key,
        description: project.description,
        status: project.status,
        target_date: project.target_date,
        owner: owner_info,
        teams: vec![], // 新创建的项目还没有关联的团队
        workspace_id: project.workspace_id,
        created_at: project.created_at,
        updated_at: project.updated_at,
    };

    let response = ApiResponse::created(project_info, "Project created successfully");
    (StatusCode::CREATED, Json(response)).into_response()
}

pub async fn get_projects(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Query(query): Query<ProjectListQuery>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 获取用户的当前工作空间
    let user = match schema::users::table
        .filter(schema::users::id.eq(claims.sub))
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

    // 确定要查询的工作空间ID
    let workspace_id = query
        .workspace_id
        .unwrap_or_else(|| user.current_workspace_id.unwrap_or(uuid::Uuid::nil()));

    // 验证用户对工作空间的访问权限
    let user_has_access = match schema::team_members::table
        .inner_join(schema::teams::table.on(schema::teams::id.eq(schema::team_members::team_id)))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .filter(schema::teams::workspace_id.eq(workspace_id))
        .select(schema::team_members::user_id)
        .first::<uuid::Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if user_has_access.is_none() {
        let response = ApiResponse::<()>::forbidden("You don't have access to this workspace");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 预处理查询参数
    let status_filter = query.status.map(|status| match status {
        crate::db::enums::ProjectStatus::Planned => "planned",
        crate::db::enums::ProjectStatus::Active => "active",
        crate::db::enums::ProjectStatus::Paused => "paused",
        crate::db::enums::ProjectStatus::Completed => "completed",
        crate::db::enums::ProjectStatus::Canceled => "canceled",
    });

    // 构建基础查询
    let mut base_query = schema::projects::table
        .filter(schema::projects::workspace_id.eq(workspace_id))
        .into_boxed();

    // 添加可选的过滤条件
    if let Some(status_str) = status_filter {
        base_query = base_query.filter(schema::projects::status.eq(status_str));
    }

    // 分页设置
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * per_page;

    // 获取总数（克隆查询以避免移动所有权）
    let total_count = match base_query.count().get_result::<i64>(&mut conn) {
        Ok(count) => count,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 重新构建查询以获取项目列表
    let mut data_query = schema::projects::table
        .filter(schema::projects::workspace_id.eq(workspace_id))
        .into_boxed();

    // 重新添加相同的过滤条件
    if let Some(status_str) = status_filter {
        data_query = data_query.filter(schema::projects::status.eq(status_str));
    }

    // 获取项目列表
    let projects = match data_query
        .order(schema::projects::created_at.desc())
        .limit(per_page)
        .offset(offset)
        .select(Project::as_select())
        .load(&mut conn)
    {
        Ok(projects) => projects,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 获取相关的用户和团队信息
    let mut project_infos = Vec::new();

    for project in projects {
        // 获取项目所有者信息
        let owner = match schema::users::table
            .filter(schema::users::id.eq(project.owner_id))
            .select(User::as_select())
            .first(&mut conn)
            .optional()
        {
            Ok(Some(user)) => UserBasicInfo {
                id: user.id,
                name: user.name,
                username: user.username,
                email: user.email,
                avatar_url: user.avatar_url,
            },
            _ => continue, // 跳过无效的项目
        };

        // 获取与项目关联的团队信息
        let teams_info = match schema::teams::table
            .inner_join(schema::issues::table.on(schema::teams::id.eq(schema::issues::team_id)))
            .filter(schema::issues::project_id.eq(project.id))
            .select(Team::as_select())
            .distinct_on(schema::teams::id)
            .load(&mut conn)
        {
            Ok(teams) => teams.into_iter().map(|team| TeamBasicInfo {
                id: team.id,
                name: team.name,
                team_key: team.team_key,
            }).collect(),
            Err(_) => vec![], // 如果获取团队信息失败，返回空列表
        };

        project_infos.push(ProjectInfo {
            id: project.id,
            name: project.name,
            project_key: project.project_key,
            description: project.description,
            status: project.status,
            target_date: project.target_date,
            owner,
            teams: teams_info,
            workspace_id: project.workspace_id,
            created_at: project.created_at,
            updated_at: project.updated_at,
        });
    }

    let project_list = ProjectListResponse {
        projects: project_infos,
        total_count,
    };

    let meta = ResponseMeta {
        request_id: None,
        pagination: Some(Pagination {
            page,
            per_page,
            total_pages: (total_count + per_page - 1) / per_page,
            has_next: page * per_page < total_count,
            has_prev: page > 1,
        }),
        total_count: Some(total_count),
        execution_time_ms: None,
    };

    let response =
        ApiResponse::success_with_meta(project_list, "Projects retrieved successfully", meta);
    (StatusCode::OK, Json(response)).into_response()
}