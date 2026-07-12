use crate::error::AppErrorResponse;
use crate::middleware::request_tracking::extract_trace_id;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use axum::http::HeaderMap;
use momentum_core::db::models::*;
use crate::middleware::auth::AuthUserInfo;
use momentum_core::services::context::RequestContext;
use momentum_core::services::workspace_members_service::WorkspaceMembersService;

// Re-export types from momentum_core for backward compatibility
pub use momentum_core::services::workspace_members::types::{
    InviteMemberRequest as WorkspaceInviteMemberRequest, MembersAndInvitations,
    WorkspaceMemberInfo,
};

#[derive(Deserialize)]
pub struct WorkspaceMemberQuery {
    pub role: Option<String>,
    pub user_id: Option<Uuid>,
}

/// 邀请成员加入工作区
pub async fn invite_member_to_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Json(payload): Json<WorkspaceInviteMemberRequest>,
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
        trace_id: extract_trace_id(&headers),
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

    match WorkspaceMembersService::invite_member(&mut conn, &ctx, &payload) {
        Ok(invitation) => {
            let response = ApiResponse::created(invitation, "Member invited successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 接受邀请
pub async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Path(invitation_id): Path<Uuid>,
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
        trace_id: extract_trace_id(&headers),
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

    match WorkspaceMembersService::accept_invitation(&mut conn, &ctx, invitation_id) {
        Ok(invitation) => {
            let response = ApiResponse::success(invitation, "Invitation accepted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 获取工作区成员列表
pub async fn get_workspace_members(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(params): Query<WorkspaceMemberQuery>,
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
        trace_id: extract_trace_id(&headers),
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

    let role_enum = params.role.as_ref().and_then(|r| match r.as_str() {
        "owner" => Some(WorkspaceMemberRole::Owner),
        "admin" => Some(WorkspaceMemberRole::Admin),
        "member" => Some(WorkspaceMemberRole::Member),
        _ => None,
    });

    match WorkspaceMembersService::get_workspace_members(
        &mut conn,
        &ctx,
        &state.asset_helper,
        workspace_id,
        role_enum,
        params.user_id,
    ) {
        Ok(members) => {
            let response =
                ApiResponse::success(members, "Workspace members retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 获取工作区成员和邀请列表
pub async fn get_workspace_members_and_invitations(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Query(params): Query<WorkspaceMemberQuery>,
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
        trace_id: extract_trace_id(&headers),
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

    let role_enum = params.role.as_ref().and_then(|r| match r.as_str() {
        "owner" => Some(WorkspaceMemberRole::Owner),
        "admin" => Some(WorkspaceMemberRole::Admin),
        "member" => Some(WorkspaceMemberRole::Member),
        _ => None,
    });

    match WorkspaceMembersService::get_members_and_invitations(
        &mut conn,
        &ctx,
        &state.asset_helper,
        role_enum,
        params.user_id,
    ) {
        Ok(result) => {
            let response = ApiResponse::success(
                result,
                "Workspace members and invitations retrieved successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 获取当前工作区成员列表
pub async fn get_current_workspace_members(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Query(params): Query<WorkspaceMemberQuery>,
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
        trace_id: extract_trace_id(&headers),
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

    let role_enum = params.role.as_ref().and_then(|r| match r.as_str() {
        "owner" => Some(WorkspaceMemberRole::Owner),
        "admin" => Some(WorkspaceMemberRole::Admin),
        "member" => Some(WorkspaceMemberRole::Member),
        _ => None,
    });

    match WorkspaceMembersService::get_current_workspace_members(
        &mut conn,
        &ctx,
        &state.asset_helper,
        role_enum,
        params.user_id,
    ) {
        Ok(result) => {
            let response = ApiResponse::success(
                result,
                "Current workspace members and invitations retrieved successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}
