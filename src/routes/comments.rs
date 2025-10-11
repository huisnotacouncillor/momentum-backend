use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use crate::db::models::api::{ApiResponse, ErrorDetail};
use crate::middleware::auth::AuthUserInfo;
use crate::services::comments_service::CommentsService;
use crate::services::context::RequestContext;

#[derive(Deserialize)]
pub struct CommentQueryParams {
    pub include_deleted: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub struct UpdateCommentRequest {
    pub content: String,
}

// 获取issue的评论列表
pub async fn get_comments(
    State(state): State<Arc<AppState>>,
    Path(issue_id): Path<Uuid>,
    Query(params): Query<CommentQueryParams>,
    auth_info: AuthUserInfo,
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

    let include_deleted = params.include_deleted.unwrap_or(false);

    match CommentsService::list_by_issue(&mut conn, &ctx, issue_id, include_deleted) {
        Ok(comments) => {
            let response = ApiResponse::success(comments, "Comments retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

// 创建评论
pub async fn create_comment(
    State(state): State<Arc<AppState>>,
    Path(issue_id): Path<Uuid>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateCommentRequest>,
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

    match CommentsService::create(&mut conn, &ctx, issue_id, payload.content) {
        Ok(comment) => {
            let response = ApiResponse::created(comment, "Comment created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

// 更新评论
pub async fn update_comment(
    State(state): State<Arc<AppState>>,
    Path(comment_id): Path<Uuid>,
    auth_info: AuthUserInfo,
    Json(payload): Json<UpdateCommentRequest>,
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

    match CommentsService::update(&mut conn, &ctx, comment_id, payload.content) {
        Ok(comment) => {
            let response = ApiResponse::success(comment, "Comment updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

// 删除评论
pub async fn delete_comment(
    State(state): State<Arc<AppState>>,
    Path(comment_id): Path<Uuid>,
    auth_info: AuthUserInfo,
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

    match CommentsService::delete(&mut conn, &ctx, comment_id) {
        Ok(()) => {
            let response = ApiResponse::<()>::ok("Comment deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

// 获取单个评论
pub async fn get_comment(
    State(state): State<Arc<AppState>>,
    Path(comment_id): Path<Uuid>,
    auth_info: AuthUserInfo,
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

    match CommentsService::get_by_id(&mut conn, &ctx, comment_id) {
        Ok(comment) => {
            let response = ApiResponse::success(comment, "Comment retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}
