use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppErrorResponse;
use crate::middleware::auth::AuthUserInfo;
use crate::state::AppState;
use momentum_core::db::models::api::{ApiResponse, ErrorDetail};
use momentum_core::db::models::plugin::NewPlugin;
use momentum_core::db::models::plugin_installation::NewPluginInstallation;
use momentum_core::db::repositories::issue_field_definitions::IssueFieldDefinitionRepo;
use momentum_core::db::repositories::plugin_installations::PluginInstallationRepo;
use momentum_core::db::repositories::plugins::PluginRepo;
use momentum_core::error::AppError;
use momentum_core::plugins::manifest::Manifest;

#[derive(Debug, Deserialize)]
pub struct InstallPluginRequest {
    pub manifest_yaml: String,
}

#[derive(Debug, Deserialize)]
pub struct EnableDisableRequest {
    pub error_message: Option<String>,
}

// POST /api/v1/plugins/install
pub async fn install_plugin(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<InstallPluginRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            return AppErrorResponse(AppError::internal("Database connection failed"))
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(ws) => ws,
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    // Parse manifest YAML
    let manifest: Manifest = match serde_yaml::from_str(&payload.manifest_yaml) {
        Ok(m) => m,
        Err(e) => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: Some("manifest_yaml".to_string()),
                code: "INVALID_MANIFEST".to_string(),
                message: format!("Failed to parse manifest: {}", e),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    // Upsert plugin
    let new_plugin = NewPlugin {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        publisher: manifest.publisher.clone(),
        manifest: serde_json::to_value(&manifest).unwrap_or_default(),
        status: "available".to_string(),
    };

    if let Err(e) = PluginRepo::upsert(&mut conn, &new_plugin) {
        return AppErrorResponse(AppError::internal(format!(
            "Failed to upsert plugin: {}",
            e
        )))
        .into_response();
    }

    // Check if already installed in this workspace
    match PluginInstallationRepo::find(&mut conn, workspace_id, &manifest.id) {
        Ok(Some(_)) => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "ALREADY_INSTALLED".to_string(),
                message: "Plugin already installed in this workspace".to_string(),
            }]);
            return (StatusCode::CONFLICT, Json(response)).into_response();
        }
        Ok(None) => {}
        Err(e) => {
            return AppErrorResponse(AppError::internal(format!(
                "Failed to check installation: {}",
                e
            )))
            .into_response();
        }
    }

    // Create installation record
    let new_inst = NewPluginInstallation {
        workspace_id,
        plugin_id: manifest.id.clone(),
        config: serde_json::json!({}),
        status: "disabled".to_string(),
    };

    match PluginInstallationRepo::insert(&mut conn, &new_inst) {
        Ok(inst) => {
            let response = ApiResponse::created(inst, "Plugin installed successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => AppErrorResponse(AppError::internal(format!(
            "Failed to create installation: {}",
            e
        )))
        .into_response(),
    }
}

// GET /api/v1/plugins (列出可用插件)
pub async fn list_plugins(
    State(state): State<Arc<AppState>>,
    _auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            return AppErrorResponse(AppError::internal("Database connection failed"))
                .into_response();
        }
    };

    match PluginRepo::list_available(&mut conn) {
        Ok(plugins) => {
            let response = ApiResponse::success(plugins, "Plugins retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => AppErrorResponse(AppError::internal(format!("Failed to list plugins: {}", e)))
            .into_response(),
    }
}

// GET /api/v1/workspaces/:wid/plugins (列出已安装插件)
pub async fn list_workspace_plugins(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
    _auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            return AppErrorResponse(AppError::internal("Database connection failed"))
                .into_response();
        }
    };

    match PluginInstallationRepo::list_by_workspace(&mut conn, workspace_id) {
        Ok(installations) => {
            let response =
                ApiResponse::success(installations, "Workspace plugins retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => AppErrorResponse(AppError::internal(format!(
            "Failed to list workspace plugins: {}",
            e
        )))
        .into_response(),
    }
}

// POST /api/v1/plugins/:inst_id/enable
pub async fn enable_plugin(
    State(state): State<Arc<AppState>>,
    Path(inst_id): Path<Uuid>,
    auth_info: AuthUserInfo,
    Json(payload): Json<EnableDisableRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            return AppErrorResponse(AppError::internal("Database connection failed"))
                .into_response();
        }
    };

    let _workspace_id = match auth_info.current_workspace_id {
        Some(ws) => ws,
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    // Check installation exists
    match PluginInstallationRepo::find_by_id(&mut conn, inst_id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: Some("inst_id".to_string()),
                code: "NOT_FOUND".to_string(),
                message: "Installation not found".to_string(),
            }]);
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(e) => {
            return AppErrorResponse(AppError::internal(format!(
                "Failed to find installation: {}",
                e
            )))
            .into_response();
        }
    }

    let now = Utc::now();
    match PluginInstallationRepo::update_status(
        &mut conn,
        inst_id,
        "enabled",
        Some(now),
        payload.error_message.as_deref(),
    ) {
        Ok(inst) => {
            let response = ApiResponse::success(inst, "Plugin enabled successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => AppErrorResponse(AppError::internal(format!(
            "Failed to enable plugin: {}",
            e
        )))
        .into_response(),
    }
}

// POST /api/v1/plugins/:inst_id/disable
pub async fn disable_plugin(
    State(state): State<Arc<AppState>>,
    Path(inst_id): Path<Uuid>,
    auth_info: AuthUserInfo,
    Json(payload): Json<EnableDisableRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            return AppErrorResponse(AppError::internal("Database connection failed"))
                .into_response();
        }
    };

    let _workspace_id = match auth_info.current_workspace_id {
        Some(ws) => ws,
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    match PluginInstallationRepo::find_by_id(&mut conn, inst_id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: Some("inst_id".to_string()),
                code: "NOT_FOUND".to_string(),
                message: "Installation not found".to_string(),
            }]);
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(e) => {
            return AppErrorResponse(AppError::internal(format!(
                "Failed to find installation: {}",
                e
            )))
            .into_response();
        }
    }

    match PluginInstallationRepo::update_status(
        &mut conn,
        inst_id,
        "disabled",
        None,
        payload.error_message.as_deref(),
    ) {
        Ok(inst) => {
            let response = ApiResponse::success(inst, "Plugin disabled successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => AppErrorResponse(AppError::internal(format!(
            "Failed to disable plugin: {}",
            e
        )))
        .into_response(),
    }
}

// DELETE /api/v1/workspaces/:wid/plugins/:pid
pub async fn uninstall_plugin(
    State(state): State<Arc<AppState>>,
    Path((workspace_id, plugin_id)): Path<(Uuid, String)>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            return AppErrorResponse(AppError::internal("Database connection failed"))
                .into_response();
        }
    };

    let _ws = match auth_info.current_workspace_id {
        Some(ws) => ws,
        None => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace selected".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    match PluginInstallationRepo::delete(&mut conn, workspace_id, &plugin_id) {
        Ok(0) => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: None,
                code: "NOT_FOUND".to_string(),
                message: "Installation not found".to_string(),
            }]);
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Ok(_) => {
            let response = ApiResponse::<()>::ok("Plugin uninstalled successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => AppErrorResponse(AppError::internal(format!(
            "Failed to uninstall plugin: {}",
            e
        )))
        .into_response(),
    }
}

// GET /api/v1/workspaces/:wid/fields
pub async fn list_workspace_fields(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
    _auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            return AppErrorResponse(AppError::internal("Database connection failed"))
                .into_response();
        }
    };

    match IssueFieldDefinitionRepo::list_by_workspace(&mut conn, workspace_id) {
        Ok(fields) => {
            let response = ApiResponse::success(fields, "Fields retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => AppErrorResponse(AppError::internal(format!("Failed to list fields: {}", e)))
            .into_response(),
    }
}
