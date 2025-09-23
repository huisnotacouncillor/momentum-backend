use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::models::*;
use crate::middleware::auth::AuthUserInfo;
use crate::services::context::RequestContext;
use crate::services::project_statuses_service::ProjectStatusesService;

#[derive(Deserialize)]
pub struct ProjectStatusQuery {
    pub name: Option<String>,
    pub category: Option<String>,
}

/// Get all project statuses for a workspace
pub async fn get_project_statuses(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Query(_params): Query<ProjectStatusQuery>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext { user_id: auth_info.user.id, workspace_id: ws, idempotency_key: None },
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    match ProjectStatusesService::list(&mut conn, &ctx) {
        Ok(list) => {
            let response = ApiResponse::success(list, "Project statuses retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct CreateProjectStatusRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub category: String,
}

/// Create a new project status
pub async fn create_project_status(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateProjectStatusRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext { user_id: auth_info.user.id, workspace_id: ws, idempotency_key: None },
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    let category_enum = match payload.category.as_str() {
        "backlog" => crate::db::models::project_status::ProjectStatusCategory::Backlog,
        "planned" => crate::db::models::project_status::ProjectStatusCategory::Planned,
        "in_progress" => crate::db::models::project_status::ProjectStatusCategory::InProgress,
        "completed" => crate::db::models::project_status::ProjectStatusCategory::Completed,
        "canceled" => crate::db::models::project_status::ProjectStatusCategory::Canceled,
        _ => crate::db::models::project_status::ProjectStatusCategory::Backlog,
    };

    let model_request = crate::db::models::project_status::CreateProjectStatusRequest {
        name: payload.name,
        description: payload.description,
        color: Some(payload.color),
        category: category_enum,
    };

    match ProjectStatusesService::create(&mut conn, &ctx, &model_request) {
        Ok(status) => {
            let response = ApiResponse::created(status, "Project status created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// Get a specific project status by ID
pub async fn get_project_status_by_id(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(status_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext { user_id: auth_info.user.id, workspace_id: ws, idempotency_key: None },
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    match ProjectStatusesService::get_by_id(&mut conn, &ctx, status_id) {
        Ok(status) => {
            let response = ApiResponse::success(status, "Project status retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct UpdateProjectStatusRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: Option<String>,
}

/// Update a project status
pub async fn update_project_status(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(status_id): Path<Uuid>,
    Json(payload): Json<UpdateProjectStatusRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext { user_id: auth_info.user.id, workspace_id: ws, idempotency_key: None },
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    match ProjectStatusesService::update(&mut conn, &ctx, status_id, &payload) {
        Ok(status) => {
            let response = ApiResponse::success(status, "Project status updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// Delete a project status
pub async fn delete_project_status(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(status_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext { user_id: auth_info.user.id, workspace_id: ws, idempotency_key: None },
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    match ProjectStatusesService::delete(&mut conn, &ctx, status_id) {
        Ok(()) => {
            let response = ApiResponse::<()>::ok("Project status deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}