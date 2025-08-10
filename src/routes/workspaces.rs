use crate::AppState;
use axum::{
    extract::{Path, State, TypedHeader},
    http::StatusCode,
    response::IntoResponse,
    Json
};
use diesel::prelude::*;
use headers::{Authorization, authorization::Bearer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::models::*;
use crate::middleware::auth::{AuthService, AuthConfig};
use crate::schema;

#[derive(Deserialize, Serialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub url_key: String,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub url_key: Option<String>,
}

// 创建工作空间
pub async fn create_workspace(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> impl IntoResponse {
    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证工作空间名称和url_key不为空
    if payload.name.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("name".to_string()),
            code: "REQUIRED".to_string(),
            message: "Workspace name is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    if payload.url_key.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("url_key".to_string()),
            code: "REQUIRED".to_string(),
            message: "Workspace url_key is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 创建工作空间
    let new_workspace = NewWorkspace {
        name: payload.name.clone(),
        url_key: payload.url_key.clone(),
    };

    let workspace_result = diesel::insert_into(schema::workspaces::table)
        .values(&new_workspace)
        .get_result::<Workspace>(&mut conn);

    match workspace_result {
        Ok(workspace) => {
            // 更新用户的当前工作空间
            let update_result = diesel::update(schema::users::table.filter(schema::users::id.eq(claims.sub)))
                .set(schema::users::current_workspace_id.eq(workspace.id))
                .execute(&mut conn);

            match update_result {
                Ok(_) => {
                    let workspace_info = WorkspaceInfo {
                        id: workspace.id,
                        name: workspace.name,
                        url_key: workspace.url_key,
                    };
                    
                    let response = ApiResponse::success(
                        Some(workspace_info),
                        "Workspace created successfully and set as current workspace"
                    );
                    (StatusCode::CREATED, Json(response)).into_response()
                }
                Err(_) => {
                    let response = ApiResponse::<()>::internal_error("Failed to set workspace as current");
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
                }
            }
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to create workspace");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

// 获取当前工作空间
pub async fn get_current_workspace(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 获取用户信息
    let user_result = schema::users::table
        .filter(schema::users::id.eq(claims.sub))
        .select(User::as_select())
        .first::<User>(&mut conn);

    match user_result {
        Ok(user) => {
            match user.current_workspace_id {
                Some(workspace_id) => {
                    // 获取工作空间信息
                    let workspace_result = schema::workspaces::table
                        .filter(schema::workspaces::id.eq(workspace_id))
                        .select(Workspace::as_select())
                        .first::<Workspace>(&mut conn);
                    
                    match workspace_result {
                        Ok(workspace) => {
                            let workspace_info = WorkspaceInfo {
                                id: workspace.id,
                                name: workspace.name,
                                url_key: workspace.url_key,
                            };
                            
                            let response = ApiResponse::success(
                                Some(workspace_info),
                                "Current workspace retrieved successfully"
                            );
                            (StatusCode::OK, Json(response)).into_response()
                        }
                        Err(_) => {
                            let response = ApiResponse::<()>::not_found("Workspace not found");
                            (StatusCode::NOT_FOUND, Json(response)).into_response()
                        }
                    }
                }
                None => {
                    let response = ApiResponse::<()>::not_found("No current workspace set");
                    (StatusCode::NOT_FOUND, Json(response)).into_response()
                }
            }
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve user");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

// 更新工作空间
pub async fn update_workspace(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(workspace_id): Path<Uuid>,
    Json(payload): Json<UpdateWorkspaceRequest>,
) -> impl IntoResponse {
    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查用户是否有权访问该工作空间
    // 这里简单检查用户当前工作空间是否是目标工作空间
    let user_result = schema::users::table
        .filter(schema::users::id.eq(claims.sub))
        .select(User::as_select())
        .first::<User>(&mut conn);

    match user_result {
        Ok(user) => {
            match user.current_workspace_id {
                Some(current_workspace_id) if current_workspace_id == workspace_id => {
                    // 检查是否提供了更新数据
                    if payload.name.is_none() && payload.url_key.is_none() {
                        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                            field: None,
                            code: "NO_UPDATE_DATA".to_string(),
                            message: "No update data provided".to_string(),
                        }]);
                        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                    }

                    // 执行更新
                    let result = if let Some(name) = &payload.name {
                        if name.trim().is_empty() {
                            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                                field: Some("name".to_string()),
                                code: "REQUIRED".to_string(),
                                message: "Workspace name cannot be empty".to_string(),
                            }]);
                            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                        }
                        
                        diesel::update(schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id)))
                            .set(schema::workspaces::name.eq(name))
                            .get_result::<Workspace>(&mut conn)
                    } else if let Some(url_key) = &payload.url_key {
                        if url_key.trim().is_empty() {
                            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                                field: Some("url_key".to_string()),
                                code: "REQUIRED".to_string(),
                                message: "Workspace url_key cannot be empty".to_string(),
                            }]);
                            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                        }
                        
                        diesel::update(schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id)))
                            .set(schema::workspaces::url_key.eq(url_key))
                            .get_result::<Workspace>(&mut conn)
                    } else {
                        // 不应该到达这里，但为了安全起见
                        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                            field: None,
                            code: "NO_UPDATE_DATA".to_string(),
                            message: "No update data provided".to_string(),
                        }]);
                        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                    };

                    match result {
                        Ok(workspace) => {
                            let workspace_info = WorkspaceInfo {
                                id: workspace.id,
                                name: workspace.name,
                                url_key: workspace.url_key,
                            };
                            
                            let response = ApiResponse::success(
                                Some(workspace_info),
                                "Workspace updated successfully"
                            );
                            (StatusCode::OK, Json(response)).into_response()
                        }
                        Err(_) => {
                            let response = ApiResponse::<()>::internal_error("Failed to update workspace");
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
                        }
                    }
                }
                _ => {
                    let response = ApiResponse::<()>::forbidden("Access denied to this workspace");
                    (StatusCode::FORBIDDEN, Json(response)).into_response()
                }
            }
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve user");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response() // 修复：移除末尾分号
        }
    }
}

// 删除工作空间
pub async fn delete_workspace(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(workspace_id): Path<Uuid>,
) -> impl IntoResponse {
    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查用户是否有权访问该工作空间
    let user_result = schema::users::table
        .filter(schema::users::id.eq(claims.sub))
        .select(User::as_select())
        .first::<User>(&mut conn);

    match user_result {
        Ok(user) => {
            match user.current_workspace_id {
                Some(current_workspace_id) if current_workspace_id == workspace_id => {
                    // 检查工作空间中是否有项目、团队等关联数据
                    // 这里简化处理，实际应用中可能需要更复杂的检查
                    
                    let delete_result = diesel::delete(
                        schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id))
                    )
                    .execute(&mut conn);

                    match delete_result {
                        Ok(0) => {
                            let response = ApiResponse::<()>::not_found("Workspace not found");
                            (StatusCode::NOT_FOUND, Json(response)).into_response()
                        }
                        Ok(_) => {
                            // 清除用户的当前工作空间
                            let _ = diesel::update(
                                schema::users::table.filter(schema::users::id.eq(claims.sub))
                            )
                            .set(schema::users::current_workspace_id.eq(None::<Uuid>))
                            .execute(&mut conn);
                            
                            let response = ApiResponse::<()>::success(
                                (), 
                                "Workspace deleted successfully"
                            );
                            (StatusCode::OK, Json(response)).into_response()
                        }
                        Err(_) => {
                            let response = ApiResponse::<()>::internal_error("Failed to delete workspace");
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
                        }
                    }
                }
                _ => {
                    let response = ApiResponse::<()>::forbidden("Access denied to this workspace");
                    (StatusCode::FORBIDDEN, Json(response)).into_response()
                }
            }
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve user");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}