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

use crate::db::models::*;
use crate::middleware::auth::AuthUserInfo;
use crate::services::context::RequestContext;
use crate::services::workflows_service::WorkflowsService;

#[derive(Deserialize)]
pub struct WorkflowQuery {
    pub name: Option<String>,
    pub is_default: Option<bool>,
}

/// Get all workflows for a team
pub async fn get_workflows(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
    Query(_params): Query<WorkflowQuery>,
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

    match WorkflowsService::list(&mut conn, &ctx, team_id) {
        Ok(workflows) => {
            let response = ApiResponse::success(workflows, "Workflows retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// Get a workflow by ID
pub async fn get_workflow_by_id(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
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

    match WorkflowsService::get_by_id(&mut conn, &ctx, workflow_id) {
        Ok(workflow) => {
            let response = ApiResponse::success(workflow, "Workflow retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub is_default: Option<bool>,
}

/// Create a new workflow
pub async fn create_workflow(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<CreateWorkflowRequest>,
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

    match WorkflowsService::create_workflow(
        &mut conn,
        &ctx,
        team_id,
        &payload.name,
        payload.description.clone(),
        payload.is_default.unwrap_or(false),
    ) {
        Ok(workflow) => {
            let response = ApiResponse::created(workflow, "Workflow created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_default: Option<bool>,
}

/// Update a workflow
pub async fn update_workflow(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
    Json(payload): Json<UpdateWorkflowRequest>,
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

    match WorkflowsService::update(&mut conn, &ctx, workflow_id, &payload) {
        Ok(workflow) => {
            let response = ApiResponse::success(workflow, "Workflow updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// Delete a workflow
pub async fn delete_workflow(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
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

    match WorkflowsService::delete(&mut conn, &ctx, workflow_id) {
        Ok(()) => {
            let response = ApiResponse::<()>::ok("Workflow deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// Get workflow states for a workflow
pub async fn get_workflow_states(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
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

    match WorkflowsService::get_states(&mut conn, &ctx, workflow_id) {
        Ok(states) => {
            let response = ApiResponse::success(states, "Workflow states retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// Get workflow states for a team's default workflow
pub async fn get_team_default_workflow_states(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
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

    match WorkflowsService::get_team_default_states(&mut conn, &ctx, team_id) {
        Ok(states) => {
            let response =
                ApiResponse::success(states, "Default workflow states retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct CreateWorkflowStateRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: String,
    pub position: i32,
    pub is_default: Option<bool>,
}

/// Create a new workflow state
pub async fn create_workflow_state(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
    Json(payload): Json<CreateWorkflowStateRequest>,
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

    let category_enum = match payload.category.as_str() {
        "backlog" => crate::db::models::workflow::WorkflowStateCategory::Backlog,
        "unstarted" => crate::db::models::workflow::WorkflowStateCategory::Unstarted,
        "started" => crate::db::models::workflow::WorkflowStateCategory::Started,
        "completed" => crate::db::models::workflow::WorkflowStateCategory::Completed,
        "canceled" => crate::db::models::workflow::WorkflowStateCategory::Canceled,
        "triage" => crate::db::models::workflow::WorkflowStateCategory::Triage,
        _ => crate::db::models::workflow::WorkflowStateCategory::Backlog,
    };

    let model_request = crate::db::models::workflow::CreateWorkflowStateRequest {
        name: payload.name,
        description: payload.description,
        color: payload.color,
        category: category_enum,
        position: payload.position,
        is_default: payload.is_default,
    };

    match WorkflowsService::add_state(&mut conn, &ctx, workflow_id, &model_request) {
        Ok(state) => {
            let response = ApiResponse::created(state, "Workflow state created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// Create a new workflow state for a team's default workflow
pub async fn create_team_default_workflow_state(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<CreateWorkflowStateRequest>,
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

    let category_enum = match payload.category.as_str() {
        "backlog" => crate::db::models::workflow::WorkflowStateCategory::Backlog,
        "unstarted" => crate::db::models::workflow::WorkflowStateCategory::Unstarted,
        "started" => crate::db::models::workflow::WorkflowStateCategory::Started,
        "completed" => crate::db::models::workflow::WorkflowStateCategory::Completed,
        "canceled" => crate::db::models::workflow::WorkflowStateCategory::Canceled,
        "triage" => crate::db::models::workflow::WorkflowStateCategory::Triage,
        _ => crate::db::models::workflow::WorkflowStateCategory::Backlog,
    };

    let team_default_request = crate::routes::workflows::CreateTeamDefaultStateRequest {
        name: payload.name,
        description: payload.description,
        color: payload.color,
        category: category_enum,
        position: payload.position,
    };

    match WorkflowsService::create_team_default_state(
        &mut conn,
        &ctx,
        team_id,
        &team_default_request,
    ) {
        Ok(state) => {
            let response = ApiResponse::created(state, "Workflow state created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct UpdateWorkflowStateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: Option<String>,
    pub position: Option<i32>,
    pub is_default: Option<bool>,
}

/// Update a workflow state for a team's default workflow
pub async fn update_team_default_workflow_state(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path((team_id, state_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateWorkflowStateRequest>,
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

    let category_enum = payload
        .category
        .as_ref()
        .and_then(|cat| match cat.as_str() {
            "backlog" => Some(crate::db::models::workflow::WorkflowStateCategory::Backlog),
            "unstarted" => Some(crate::db::models::workflow::WorkflowStateCategory::Unstarted),
            "started" => Some(crate::db::models::workflow::WorkflowStateCategory::Started),
            "completed" => Some(crate::db::models::workflow::WorkflowStateCategory::Completed),
            "canceled" => Some(crate::db::models::workflow::WorkflowStateCategory::Canceled),
            "triage" => Some(crate::db::models::workflow::WorkflowStateCategory::Triage),
            _ => None,
        });

    let team_default_request = crate::routes::workflows::UpdateTeamDefaultStateRequest {
        name: payload.name,
        description: payload.description,
        color: payload.color,
        category: category_enum,
        position: payload.position,
    };

    match WorkflowsService::update_team_default_state(
        &mut conn,
        &ctx,
        team_id,
        state_id,
        &team_default_request,
    ) {
        Ok(state) => {
            let response = ApiResponse::success(state, "Workflow state updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

/// Get available workflow transitions for an issue
pub async fn get_issue_transitions(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(issue_id): Path<Uuid>,
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

    match WorkflowsService::get_issue_transitions(&mut conn, &ctx, issue_id) {
        Ok(transitions) => {
            let response =
                ApiResponse::success(transitions, "Available transitions retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct CreateTeamDefaultStateRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: crate::db::models::workflow::WorkflowStateCategory,
    pub position: i32,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateTeamDefaultStateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: Option<crate::db::models::workflow::WorkflowStateCategory>,
    pub position: Option<i32>,
}

#[derive(Deserialize, Serialize)]
pub struct IssueTransition {
    pub from_state_id: Option<uuid::Uuid>,
    pub to_state_id: uuid::Uuid,
    pub to_state_name: String,
    pub to_state_color: Option<String>,
}
