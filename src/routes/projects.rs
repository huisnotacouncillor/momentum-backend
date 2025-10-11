use crate::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::models::*;
use crate::middleware::auth::AuthUserInfo;
use crate::services::context::RequestContext;
use crate::services::projects_service::ProjectsService;

#[derive(Deserialize)]
pub struct ProjectQuery {
    pub search: Option<String>,
    pub owner_id: Option<Uuid>,
}

#[derive(Deserialize, Serialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub target_date: Option<chrono::NaiveDate>,
    pub project_status_id: Option<Uuid>,
    pub priority: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub roadmap_id: Option<Uuid>,
    pub target_date: Option<chrono::NaiveDate>,
    pub project_status_id: Option<Uuid>,
    pub priority: Option<String>,
}

/// 创建项目
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext {
            user_id: auth_info.user.id,
            workspace_id: ws,
            idempotency_key: None,
        },
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    let create_req = crate::db::models::project::CreateProjectRequest {
        name: payload.name,
        project_key: payload.project_key,
        description: payload.description,
        target_date: payload.target_date,
        project_status_id: payload.project_status_id,
        priority: payload.priority.map(|p| p.parse().unwrap_or_default()),
        roadmap_id: None, // TODO: Add roadmap_id to route request
    };
    match ProjectsService::create(&mut conn, &ctx, &create_req) {
        Ok(project) => {
            let response = ApiResponse::created(project, "Project created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 获取项目列表
pub async fn get_projects(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Query(params): Query<ProjectQuery>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext {
            user_id: auth_info.user.id,
            workspace_id: ws,
            idempotency_key: None,
        },
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    match ProjectsService::list_infos(
        &mut conn,
        &ctx,
        &state.asset_helper,
        params.search,
        params.owner_id,
    ) {
        Ok(project_list_response) => {
            let response =
                ApiResponse::success(project_list_response, "Projects retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 更新项目
pub async fn update_project(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<UpdateProjectRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext {
            user_id: auth_info.user.id,
            workspace_id: ws,
            idempotency_key: None,
        },
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    let priority_enum = payload.priority.as_ref().and_then(|p| match p.as_str() {
        "none" => Some(crate::db::enums::ProjectPriority::None),
        "low" => Some(crate::db::enums::ProjectPriority::Low),
        "medium" => Some(crate::db::enums::ProjectPriority::Medium),
        "high" => Some(crate::db::enums::ProjectPriority::High),
        "urgent" => Some(crate::db::enums::ProjectPriority::Urgent),
        _ => None,
    });

    let model_request = crate::db::models::project::UpdateProjectRequest {
        name: payload.name,
        description: payload.description,
        roadmap_id: payload.roadmap_id.map(Some),
        target_date: payload.target_date.map(Some),
        project_status_id: payload.project_status_id,
        priority: priority_enum,
    };

    match ProjectsService::update(
        &mut conn,
        &ctx,
        &state.asset_helper,
        project_id,
        &model_request,
    ) {
        Ok(project) => {
            let response = ApiResponse::success(project, "Project updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 删除项目
pub async fn delete_project(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(project_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext {
            user_id: auth_info.user.id,
            workspace_id: ws,
            idempotency_key: None,
        },
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    match ProjectsService::delete(&mut conn, &ctx, project_id) {
        Ok(()) => {
            let response = ApiResponse::success((), "Project deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}
