use crate::error::AppErrorResponse;
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

use crate::middleware::auth::AuthUserInfo;
use momentum_core::db::models::*;
use momentum_core::db::repositories::notifications::NotificationsRepo;

#[derive(Debug, Deserialize)]
pub struct NotificationListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 获取当前用户的通知列表
pub async fn get_notifications(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Query(params): Query<NotificationListQuery>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let user_id = auth_info.user.id;
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    match NotificationsRepo::list_by_user(&mut conn, user_id, limit, offset) {
        Ok(notifications) => {
            let response = ApiResponse::success(notifications, "Notifications retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(momentum_core::AppError::Database(err)).into_response(),
    }
}

/// 标记单个通知为已读
pub async fn mark_notification_read(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(notification_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let user_id = auth_info.user.id;

    match NotificationsRepo::mark_as_read(&mut conn, notification_id, user_id) {
        Ok(notification) => {
            let response = ApiResponse::success(notification, "Notification marked as read");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(momentum_core::AppError::Database(err)).into_response(),
    }
}

/// 标记所有通知为已读
pub async fn mark_all_notifications_read(
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

    let user_id = auth_info.user.id;

    match NotificationsRepo::mark_all_as_read(&mut conn, user_id) {
        Ok(count) => {
            let response = ApiResponse::success(serde_json::json!({ "count": count }), "All notifications marked as read");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(momentum_core::AppError::Database(err)).into_response(),
    }
}

/// 获取未读通知数量
pub async fn get_unread_count(
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

    let user_id = auth_info.user.id;

    match NotificationsRepo::unread_count(&mut conn, user_id) {
        Ok(count) => {
            let response = ApiResponse::success(serde_json::json!({ "unread_count": count }), "Unread count retrieved");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(momentum_core::AppError::Database(err)).into_response(),
    }
}