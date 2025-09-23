use crate::AppState;
use crate::db::enums::IssuePriority;
use crate::db::models::api::{ApiResponse, ErrorDetail};
use crate::middleware::auth::AuthUserInfo;
use crate::services::context::RequestContext;
use crate::services::issues_service::{IssuesService, IssueFilters};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct IssueQueryParams {
    pub team_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub priority: Option<String>,
    pub search: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Uuid,
    pub priority: Option<IssuePriority>,
    pub assignee_id: Option<Uuid>,
    pub reporter_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
    pub cycle_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub priority: Option<IssuePriority>,
    pub assignee_id: Option<Uuid>,
    pub reporter_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
}

// 获取问题列表
pub async fn get_issues(
    State(state): State<Arc<AppState>>,
    Query(params): Query<IssueQueryParams>,
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
            idempotency_key: None
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
    };

    match IssuesService::list(&mut conn, &ctx, &filters) {
        Ok(issues) => {
            let response = ApiResponse::success(issues, "Issues retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
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
            idempotency_key: None
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

    match IssuesService::create(&mut conn, &ctx, &payload) {
        Ok(issue) => {
            let response = ApiResponse::created(issue, "Issue created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
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
            idempotency_key: None
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

    match IssuesService::update(&mut conn, &ctx, issue_id, &payload) {
        Ok(issue) => {
            let response = ApiResponse::success(issue, "Issue updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
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
            idempotency_key: None
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

    match IssuesService::delete(&mut conn, &ctx, issue_id) {
        Ok(()) => {
            let response = ApiResponse::<()>::ok("Issue deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
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
            idempotency_key: None
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

    match IssuesService::get_by_id(&mut conn, &ctx, issue_id) {
        Ok(issue) => {
            let response = ApiResponse::success(issue, "Issue retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}
