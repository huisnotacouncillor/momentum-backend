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
use momentum_core::services::invitations_service::InvitationsService;

// Re-export types from momentum_core for backward compatibility
pub use momentum_core::services::invitations::types::{InvitationInfo, InviteMemberRequest};

#[derive(Deserialize)]
pub struct InvitationQuery {
    pub status: Option<String>,
    pub email: Option<String>,
}

/// 邀请用户加入当前工作区
///
/// 权限要求: 当前用户必须是工作区的Owner或Admin
pub async fn invite_member(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Json(payload): Json<InviteMemberRequest>,
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

    match InvitationsService::invite_members(&mut conn, &ctx, &payload) {
        Ok(result) => {
            let response = ApiResponse::created(result, "Members invited successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 获取当前用户的邀请列表
pub async fn get_user_invitations(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Query(params): Query<InvitationQuery>,
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

    let status_enum = params.status.as_ref().and_then(|s| match s.as_str() {
        "pending" => Some(momentum_core::db::models::invitation::InvitationStatus::Pending),
        "accepted" => Some(momentum_core::db::models::invitation::InvitationStatus::Accepted),
        "declined" => Some(momentum_core::db::models::invitation::InvitationStatus::Declined),
        "cancelled" => Some(momentum_core::db::models::invitation::InvitationStatus::Cancelled),
        _ => None,
    });

    match InvitationsService::get_user_invitations(
        &mut conn,
        &ctx,
        &state.asset_helper,
        status_enum,
        params.email,
    ) {
        Ok(invitations) => {
            let response =
                ApiResponse::success(invitations, "User invitations retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 获取特定邀请的详细信息
pub async fn get_invitation_by_id(
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

    match InvitationsService::get_by_id(&mut conn, &ctx, &state.asset_helper, invitation_id) {
        Ok(invitation) => {
            let response = ApiResponse::success(invitation, "Invitation retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
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

    match InvitationsService::accept(&mut conn, &ctx, invitation_id) {
        Ok(invitation) => {
            let response = ApiResponse::success(invitation, "Invitation accepted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 拒绝邀请
pub async fn decline_invitation(
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

    match InvitationsService::decline(&mut conn, &ctx, invitation_id) {
        Ok(invitation) => {
            let response = ApiResponse::success(invitation, "Invitation declined successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// 撤销邀请（由邀请人操作）
///
/// 权限要求: 当前用户必须是邀请人或工作区的Owner/Admin
pub async fn revoke_invitation(
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

    match InvitationsService::revoke(&mut conn, &ctx, invitation_id) {
        Ok(invitation) => {
            let response = ApiResponse::success(invitation, "Invitation revoked successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}
