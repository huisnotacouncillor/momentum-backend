use crate::error::AppErrorResponse;
use crate::middleware::request_tracking::extract_trace_id;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
// use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use axum::http::HeaderMap;
use momentum_core::db::enums::LabelLevel;
use momentum_core::db::models::*;
use crate::middleware::auth::AuthUserInfo;
// use crate::schema; // no longer needed in handlers after service extraction
use momentum_core::services::context::RequestContext;
use momentum_core::services::labels_service::LabelsService;

// Re-export types from momentum_core for backward compatibility
pub use momentum_core::services::labels::types::{CreateLabelRequest, UpdateLabelRequest};

#[derive(Deserialize)]
pub struct LabelQuery {
    pub name: Option<String>,
    pub level: Option<LabelLevel>,
}

// 获取标签列表
pub async fn get_labels(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Query(params): Query<LabelQuery>,
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

    match LabelsService::list(&mut conn, &ctx, params.name, params.level) {
        Ok(labels) => {
            let response = ApiResponse::success(labels, "Labels retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 创建标签
pub async fn create_label(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Json(payload): Json<CreateLabelRequest>,
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

    match LabelsService::create(&mut conn, &ctx, &payload) {
        Ok(label) => {
            let response = ApiResponse::created(label, "Label created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 更新标签
pub async fn update_label(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Path(label_id): Path<Uuid>,
    Json(payload): Json<UpdateLabelRequest>,
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

    match LabelsService::update(&mut conn, &ctx, label_id, &payload) {
        Ok(label) => {
            let response = ApiResponse::success(label, "Label updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 删除标签
pub async fn delete_label(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Path(label_id): Path<Uuid>,
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

    match LabelsService::delete(&mut conn, &ctx, label_id) {
        Ok(()) => {
            let response = ApiResponse::<()>::ok("Label deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}
