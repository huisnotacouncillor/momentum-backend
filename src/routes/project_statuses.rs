use crate::db::{DbPool, models::project_status::{CreateProjectStatusRequest, ProjectStatus, ProjectStatusInfo}};
use crate::middleware::auth::AuthUserInfo;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;
use crate::db::models::ApiResponse;

/// Get all project statuses for a workspace
pub async fn get_project_statuses(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    match crate::schema::project_statuses::table
        .filter(crate::schema::project_statuses::workspace_id.eq(workspace_id))
        .load::<ProjectStatus>(&mut conn)
    {
        Ok(results) => {
            let list: Vec<ProjectStatusInfo> = results
                .into_iter()
                .map(|status| ProjectStatusInfo {
                    id: status.id,
                    name: status.name,
                    description: status.description,
                    color: status.color,
                    category: status.category,
                    created_at: status.created_at,
                    updated_at: status.updated_at,
                })
                .collect();
            let response = ApiResponse::success(list, "Project status retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to load project statuses"
            })),
        )
            .into_response(),
    }
}

/// Create a new project status
pub async fn create_project_status(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Json(request): Json<CreateProjectStatusRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    let new_status = crate::db::models::project_status::NewProjectStatus {
        name: request.name,
        description: request.description,
        color: request.color,
        category: request.category,
        workspace_id,
    };

    match diesel::insert_into(crate::schema::project_statuses::table)
        .values(&new_status)
        .get_result::<ProjectStatus>(&mut conn)
    {
        Ok(status) => {
            let response = ProjectStatusInfo {
                id: status.id,
                name: status.name,
                description: status.description,
                color: status.color,
                category: status.category,
                created_at: status.created_at,
                updated_at: status.updated_at,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to create project status"
            })),
        )
            .into_response(),
    }
}

/// Get a specific project status by ID
pub async fn get_project_status_by_id(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(status_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    match crate::schema::project_statuses::table
        .filter(crate::schema::project_statuses::id.eq(status_id))
        .filter(crate::schema::project_statuses::workspace_id.eq(workspace_id))
        .first::<ProjectStatus>(&mut conn)
    {
        Ok(status) => {
            let response = ProjectStatusInfo {
                id: status.id,
                name: status.name,
                description: status.description,
                color: status.color,
                category: status.category,
                created_at: status.created_at,
                updated_at: status.updated_at,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(diesel::result::Error::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Project status not found"
            })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to get project status"
            })),
        )
            .into_response(),
    }
}

/// Update a project status
pub async fn update_project_status(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(status_id): Path<Uuid>,
    Json(request): Json<CreateProjectStatusRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    let target = crate::schema::project_statuses::table
        .filter(crate::schema::project_statuses::id.eq(status_id))
        .filter(crate::schema::project_statuses::workspace_id.eq(workspace_id));

    match diesel::update(target)
        .set((
            crate::schema::project_statuses::name.eq(request.name),
            crate::schema::project_statuses::description.eq(request.description),
            crate::schema::project_statuses::color.eq(request.color),
            crate::schema::project_statuses::category.eq(request.category),
        ))
        .get_result::<ProjectStatus>(&mut conn)
    {
        Ok(updated_status) => {
            let response = ProjectStatusInfo {
                id: updated_status.id,
                name: updated_status.name,
                description: updated_status.description,
                color: updated_status.color,
                category: updated_status.category,
                created_at: updated_status.created_at,
                updated_at: updated_status.updated_at,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(diesel::result::Error::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Project status not found"
            })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to update project status"
            })),
        )
            .into_response(),
    }
}

/// Delete a project status
pub async fn delete_project_status(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(status_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    match diesel::delete(
        crate::schema::project_statuses::table
            .filter(crate::schema::project_statuses::id.eq(status_id))
            .filter(crate::schema::project_statuses::workspace_id.eq(workspace_id)),
    )
    .execute(&mut conn)
    {
        Ok(0) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Project status not found"
            })),
        )
            .into_response(),
        Ok(_) => (StatusCode::NO_CONTENT, Json(serde_json::Value::Null)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to delete project status"
            })),
        )
            .into_response(),
    }
}