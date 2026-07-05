use crate::error::AppErrorResponse;
use crate::state::AppState;
use momentum_core::db::enums::IssuePriority;
use momentum_core::db::models::api::{ApiResponse, ErrorDetail};
use crate::middleware::auth::AuthUserInfo;
use momentum_core::services::context::RequestContext;
use momentum_core::services::issues_service::{IssueFilters, IssuesService};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

// Re-export types from momentum_core for backward compatibility
pub use momentum_core::services::issues::types::{CreateIssueRequest, UpdateIssueRequest};

#[derive(Deserialize)]
pub struct IssueQueryParams {
    pub team_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub priority: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

// 获取问题列表
pub async fn get_issues(
    State(state): State<Arc<AppState>>,
    Query(params): Query<IssueQueryParams>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let ctx = match auth_info.current_workspace_id {
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

    // Parse priority if provided
    let priority = if let Some(priority_str) = params.priority {
        match priority_str.as_str() {
            "none" => Some(IssuePriority::None),
            "low" => Some(IssuePriority::Low),
            "medium" => Some(IssuePriority::Medium),
            "high" => Some(IssuePriority::High),
            "urgent" => Some(IssuePriority::Urgent),
            _ => {
                let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                    field: Some("priority".to_string()),
                    code: "INVALID_PRIORITY".to_string(),
                    message: "Invalid priority value".to_string(),
                }]);
                return (StatusCode::BAD_REQUEST, Json(response)).into_response();
            }
        }
    } else {
        None
    };

    let filters = IssueFilters {
        team_id: params.team_id,
        project_id: params.project_id,
        assignee_id: params.assignee_id,
        priority,
        search: params.search,
        limit: params.limit,
        cursor: params.cursor,
    };

    let service = IssuesService::new();

    // P1.3 修复：使用 spawn_blocking 包装同步 DB 调用，避免阻塞 tokio 工作线程
    let result = momentum_core::db::run_db(&state.db, move |conn| {
        service.list(conn, &ctx, &filters)
    })
    .await;

    match result {
        Ok(paginated) => {
            #[derive(serde::Serialize)]
            struct PaginatedResponse {
                items: Vec<momentum_core::db::models::issue::IssueResponse>,
                next_cursor: Option<String>,
                has_more: bool,
            }
            let resp = PaginatedResponse {
                items: paginated.items,
                next_cursor: paginated.next_cursor,
                has_more: paginated.has_more,
            };
            let response = ApiResponse::success(resp, "Issues retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 创建问题
pub async fn create_issue(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateIssueRequest>,
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

    let service = IssuesService::new();
    match service.create(&mut conn, &ctx, &payload).await {
        Ok(issue) => {
            let response = ApiResponse::created(issue, "Issue created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 更新问题
pub async fn update_issue(
    State(state): State<Arc<AppState>>,
    Path(issue_id): Path<Uuid>,
    auth_info: AuthUserInfo,
    Json(payload): Json<UpdateIssueRequest>,
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

    let service = IssuesService::new();
    match service.update(&mut conn, &ctx, issue_id, &payload).await {
        Ok(issue) => {
            let response = ApiResponse::success(issue, "Issue updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 删除问题
pub async fn delete_issue(
    State(state): State<Arc<AppState>>,
    Path(issue_id): Path<Uuid>,
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

    let service = IssuesService::new();
    match service.delete(&mut conn, &ctx, issue_id) {
        Ok(()) => {
            let response = ApiResponse::<()>::ok("Issue deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 获取单个问题
pub async fn get_issue(
    State(state): State<Arc<AppState>>,
    Path(issue_id): Path<Uuid>,
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

    let service = IssuesService::new();
    match service.get_by_id(&mut conn, &ctx, issue_id) {
        Ok(issue) => {
            let response = ApiResponse::success(issue, "Issue retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}
