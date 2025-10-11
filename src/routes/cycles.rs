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

use crate::db::enums::*;
use crate::db::models::*;
use crate::middleware::auth::AuthUserInfo;
use crate::services::context::RequestContext;
use crate::services::cycles_service::CyclesService;

// 请求体定义
#[derive(Deserialize)]
pub struct CreateCycleRequest {
    pub team_id: Uuid,
    pub name: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub description: Option<String>,
    pub goal: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateCycleRequest {
    pub team_id: Option<Uuid>,
    pub name: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub goal: Option<String>,
}

#[derive(Deserialize)]
pub struct AssignIssuesToCycleRequest {
    pub issue_ids: Vec<Uuid>,
}

#[derive(Deserialize)]
pub struct CycleIssuesQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub status: Option<String>,
}

#[derive(Serialize)]
pub struct CycleStats {
    pub id: Uuid,
    pub name: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub status: CycleStatus,
    pub total_issues: i64,
    pub completed_issues: i64,
    pub in_progress_issues: i64,
    pub todo_issues: i64,
    pub completion_rate: f64,
    pub days_remaining: i32,
    pub is_overdue: bool,
}

#[derive(Serialize)]
pub struct CycleWithIssues {
    pub cycle: Cycle,
    pub issues: Vec<Issue>,
    pub stats: CycleStats,
}

#[derive(Serialize)]
pub struct PaginatedIssues {
    pub issues: Vec<Issue>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

/// 创建 Cycle
pub async fn create_cycle(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateCycleRequest>,
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

    match CyclesService::create(&mut conn, &ctx, &payload) {
        Ok(cycle) => {
            let response = ApiResponse::created(cycle, "Cycle created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

#[derive(Deserialize)]
pub struct CycleQuery {
    pub team_id: Option<Uuid>,
    pub status: Option<String>,
}

/// 获取 cycles 列表
pub async fn get_cycles(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Query(_params): Query<CycleQuery>,
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

    match CyclesService::list(&mut conn, &ctx) {
        Ok(cycles) => {
            let response = ApiResponse::success(cycles, "Cycles retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 获取指定 cycle
pub async fn get_cycle_by_id(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
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

    match CyclesService::get_by_id(&mut conn, &ctx, cycle_id) {
        Ok(cycle) => {
            let response = ApiResponse::success(cycle, "Cycle retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 更新指定 cycle
pub async fn update_cycle(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
    Json(payload): Json<UpdateCycleRequest>,
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

    match CyclesService::update(&mut conn, &ctx, cycle_id, &payload) {
        Ok(cycle) => {
            let response = ApiResponse::success(cycle, "Cycle updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 删除指定 cycle
pub async fn delete_cycle(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
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

    match CyclesService::delete(&mut conn, &ctx, cycle_id) {
        Ok(()) => {
            let response = ApiResponse::<()>::ok("Cycle deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 获取周期统计信息
pub async fn get_cycle_stats(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
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

    match CyclesService::get_stats(&mut conn, &ctx, cycle_id) {
        Ok(stats) => {
            let response = ApiResponse::success(stats, "Cycle statistics retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 获取周期内的 Issues 列表
pub async fn get_cycle_issues(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
    Query(query): Query<CycleIssuesQuery>,
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

    match CyclesService::get_issues(
        &mut conn,
        &ctx,
        cycle_id,
        query.page,
        query.limit,
        query.status,
    ) {
        Ok(issues) => {
            let response = ApiResponse::success(issues, "Cycle issues retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 将 Issues 分配到周期
pub async fn assign_issues_to_cycle(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
    Json(payload): Json<AssignIssuesToCycleRequest>,
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

    match CyclesService::assign_issues(&mut conn, &ctx, cycle_id, &payload.issue_ids) {
        Ok(_count) => {
            let response = ApiResponse::success(
                Some(format!(
                    "Successfully assigned {} issues to cycle",
                    payload.issue_ids.len()
                )),
                "Issues assigned to cycle successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 从周期中移除 Issues
pub async fn remove_issues_from_cycle(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
    Json(payload): Json<AssignIssuesToCycleRequest>,
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

    match CyclesService::remove_issues(&mut conn, &ctx, cycle_id, &payload.issue_ids) {
        Ok(_count) => {
            let response = ApiResponse::success(
                Some(format!(
                    "Successfully removed {} issues from cycle",
                    payload.issue_ids.len()
                )),
                "Issues removed from cycle successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// 自动更新周期状态
pub async fn update_cycle_status_auto(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match CyclesService::auto_update_status(&mut conn) {
        Ok(_counts) => {
            let message = format!(
                "Auto-updated {} cycles to active and {} cycles to completed",
                0, 0
            );
            let response =
                ApiResponse::success(Some(message), "Cycle statuses updated automatically");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}
