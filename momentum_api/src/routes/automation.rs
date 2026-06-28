use crate::error::AppErrorResponse;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use momentum_core::db::models::api::ApiResponse;
use momentum_core::db::models::automation::{NewAutomationRule, UpdateAutomationRule};
use momentum_core::db::repositories::automation::AutomationRepo;

pub async fn list_rules(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match AutomationRepo::list_by_workspace(&mut conn, workspace_id) {
        Ok(rules) => {
            let response = ApiResponse::success(rules, "Automation rules retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

pub async fn get_rule(
    State(state): State<Arc<AppState>>,
    Path((_workspace_id, rule_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match AutomationRepo::find_by_id(&mut conn, rule_id) {
        Ok(rule) => {
            let response = ApiResponse::success(rule, "Automation rule retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

pub async fn create_rule(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
    Json(payload): Json<NewAutomationRule>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // Override workspace_id from URL path
    let rule_with_workspace = NewAutomationRule {
        workspace_id,
        team_id: payload.team_id,
        name: payload.name,
        description: payload.description,
        is_enabled: payload.is_enabled,
        trigger_type: payload.trigger_type,
        trigger_config: payload.trigger_config,
        conditions: payload.conditions,
        actions: payload.actions,
    };

    match AutomationRepo::create(&mut conn, &rule_with_workspace) {
        Ok(rule) => {
            let response = ApiResponse::created(rule, "Automation rule created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

pub async fn update_rule(
    State(state): State<Arc<AppState>>,
    Path((_workspace_id, rule_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateAutomationRule>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match AutomationRepo::update(&mut conn, rule_id, &payload) {
        Ok(rule) => {
            let response = ApiResponse::success(rule, "Automation rule updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

pub async fn delete_rule(
    State(state): State<Arc<AppState>>,
    Path((_workspace_id, rule_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match AutomationRepo::delete(&mut conn, rule_id) {
        Ok(()) => {
            let response = ApiResponse::<()>::ok("Automation rule deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}
