use crate::error::AppErrorResponse;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use momentum_core::db::models::*;
use crate::middleware::auth::AuthUserInfo;
use momentum_core::services::context::RequestContext;
use momentum_core::services::workspaces_service::WorkspacesService;

// Re-export types from momentum_core for backward compatibility
pub use momentum_core::services::workspaces::types::UpdateWorkspaceRequest;

#[derive(Deserialize, Serialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub url_key: String,
    pub logo_url: Option<String>,
}

/// 创建工作空间
pub async fn create_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let _ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext {
            user_id: auth_info.user.id,
            workspace_id: ws,
            idempotency_key: None,
        trace_id: "unknown".to_string(),
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

    match WorkspacesService::create(
        &mut conn,
        &payload.name,
        &payload.url_key,
        payload.logo_url.clone(),
    ) {
        Ok(workspace) => {
            let response = ApiResponse::created(workspace, "Workspace created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 获取当前工作空间
pub async fn get_current_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let _ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext {
            user_id: auth_info.user.id,
            workspace_id: ws,
            idempotency_key: None,
        trace_id: "unknown".to_string(),
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

    match WorkspacesService::get_current(&mut conn, &_ctx, &state.asset_helper) {
        Ok(workspace) => {
            let response =
                ApiResponse::success(workspace, "Current workspace retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 更新工作空间
pub async fn update_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(workspace_id): Path<Uuid>,
    Json(payload): Json<UpdateWorkspaceRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let _ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext {
            user_id: auth_info.user.id,
            workspace_id: ws,
            idempotency_key: None,
        trace_id: "unknown".to_string(),
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

    match WorkspacesService::update(&mut conn, &_ctx, workspace_id, &payload) {
        Ok(workspace) => {
            let response = ApiResponse::success(workspace, "Workspace updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 删除工作空间
pub async fn delete_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(workspace_id): Path<Uuid>,
) -> impl IntoResponse {
    // P0 修复：只有 Owner 才能删除工作区
    if let Err(e) =
        crate::middleware::permission::require_owner(&state, workspace_id, &auth_info).await
    {
        let (status, response) = e.to_http_response();
        return (
            StatusCode::from_u16(status).unwrap_or(StatusCode::FORBIDDEN),
            Json(response),
        )
            .into_response();
    }

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let _ctx = match auth_info.current_workspace_id {
        Some(ws) => RequestContext {
            user_id: auth_info.user.id,
            workspace_id: ws,
            idempotency_key: None,
        trace_id: "unknown".to_string(),
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

    match WorkspacesService::delete(&mut conn, &_ctx, workspace_id) {
        Ok(()) => {
            let response = ApiResponse::success((), "Workspace deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}
