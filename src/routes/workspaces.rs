use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::models::*;
use crate::middleware::auth::AuthUserInfo;
use crate::schema;

#[derive(Deserialize, Serialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub url_key: String,
    pub logo_url: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub url_key: Option<String>,
    pub logo_url: Option<String>,
}

// 创建工作空间
pub async fn create_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> impl IntoResponse {
    let user_id = auth_info.user.id;
    let _workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证工作空间名称不为空
    if payload.name.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("name".to_string()),
            code: "REQUIRED".to_string(),
            message: "Workspace name is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 验证 url_key 不为空且格式正确
    if payload.url_key.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("url_key".to_string()),
            code: "REQUIRED".to_string(),
            message: "Workspace url_key is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 检查 url_key 格式
    if !payload.url_key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("url_key".to_string()),
            code: "INVALID_FORMAT".to_string(),
            message: "Workspace url_key can only contain letters, numbers, hyphens, and underscores".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 创建工作空间
    let new_workspace = NewWorkspace {
        name: payload.name.clone(),
        url_key: payload.url_key.clone(),
        logo_url: payload.logo_url.clone(),
    };

    let workspace = match diesel::insert_into(schema::workspaces::table)
        .values(&new_workspace)
        .get_result::<Workspace>(&mut conn)
    {
        Ok(workspace) => workspace,
        Err(e) => {
            // 检查是否是唯一性约束违反错误
            if e.to_string().contains("url_key") {
                let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                    field: Some("url_key".to_string()),
                    code: "DUPLICATE".to_string(),
                    message: "Workspace with this url_key already exists".to_string(),
                }]);
                return (StatusCode::BAD_REQUEST, Json(response)).into_response();
            } else {
                let response = ApiResponse::<()>::internal_error("Failed to create workspace");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    };

    // 创建工作空间成员关系
    let new_workspace_member = NewWorkspaceMember {
        user_id,
        workspace_id: workspace.id,
        role: WorkspaceMemberRole::Owner,
    };

    if let Err(_) = diesel::insert_into(schema::workspace_members::table)
        .values(&new_workspace_member)
        .execute(&mut conn)
    {
        let response = ApiResponse::<()>::internal_error("Failed to create workspace member relationship");
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
    }

    // 更新用户的 current_workspace_id
    let _updated_user = match diesel::update(schema::users::table.filter(schema::users::id.eq(user_id)))
        .set(schema::users::current_workspace_id.eq(workspace.id))
        .get_result::<User>(&mut conn)
    {
        Ok(user) => user,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update user's current workspace");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(workspace),
        "Workspace created successfully",
    );
    (StatusCode::CREATED, Json(response)).into_response()
}

// 获取当前工作空间
pub async fn get_current_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let workspace_id = auth_info.current_workspace_id.unwrap();
    let asset_helper = &state.asset_helper;

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    use crate::schema::workspaces::dsl::*;
    let mut workspace = match workspaces
        .filter(id.eq(workspace_id))
        .select(Workspace::as_select())
        .first(&mut conn)
    {
        Ok(workspace) => workspace,
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Workspace not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };
    let processed_logo_url = workspace.get_processed_logo_url(&asset_helper);
    workspace.logo_url = processed_logo_url;

    let response = ApiResponse::success(
        Some(workspace),
        "Current workspace retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

// 更新工作空间
pub async fn update_workspace(
    State(state): State<Arc<AppState>>,
    _auth_info: AuthUserInfo,
    Path(workspace_id): Path<Uuid>,
    Json(payload): Json<UpdateWorkspaceRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查工作空间是否存在
    use crate::schema::workspaces::dsl::*;
    let existing_workspace = match workspaces
        .filter(id.eq(workspace_id))
        .select(Workspace::as_select())
        .first::<Workspace>(&mut conn)
    {
        Ok(workspace) => workspace,
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Workspace not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    // 构建更新数据
    let mut name_update = None;
    let mut url_key_update = None;
    let mut logo_url_update = None;

    if let Some(ref workspace_name) = payload.name {
        if workspace_name.trim().is_empty() {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: Some("name".to_string()),
                code: "REQUIRED".to_string(),
                message: "Workspace name cannot be empty".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
        name_update = Some(schema::workspaces::name.eq(workspace_name));
    }

    if let Some(ref workspace_url_key) = payload.url_key {
        if workspace_url_key.trim().is_empty() {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: Some("url_key".to_string()),
                code: "REQUIRED".to_string(),
                message: "Workspace url_key cannot be empty".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }

        // 检查 url_key 格式
        if !workspace_url_key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: Some("url_key".to_string()),
                code: "INVALID_FORMAT".to_string(),
                message: "Workspace url_key can only contain letters, numbers, hyphens, and underscores".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
        url_key_update = Some(schema::workspaces::url_key.eq(workspace_url_key));
    }

    if let Some(ref logo_url_value) = payload.logo_url {
        logo_url_update = Some(schema::workspaces::logo_url.eq(logo_url_value));
    }

    // 如果没有要更新的字段，直接返回
    if name_update.is_none() && url_key_update.is_none() && logo_url_update.is_none() {
        let response = ApiResponse::success(
            Some(existing_workspace),
            "Workspace retrieved successfully",
        );
        return (StatusCode::OK, Json(response)).into_response();
    }

    // 执行更新 - 需要分别处理不同的字段组合
    let updated_workspace = if let Some(name_update) = name_update {
        if let Some(url_key_update) = url_key_update {
            if let Some(logo_url_update) = logo_url_update {
                // 同时更新 name, url_key 和 logo_url
                match diesel::update(schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id)))
                    .set((name_update, url_key_update, logo_url_update))
                    .get_result::<Workspace>(&mut conn)
                {
                    Ok(workspace) => workspace,
                    Err(e) => {
                        // 检查是否是唯一性约束违反错误
                        if e.to_string().contains("url_key") {
                            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                                field: Some("url_key".to_string()),
                                code: "DUPLICATE".to_string(),
                                message: "Workspace with this url_key already exists".to_string(),
                            }]);
                            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                        } else {
                            let response = ApiResponse::<()>::internal_error("Failed to update workspace");
                            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                        }
                    }
                }
            } else {
                // 更新 name 和 url_key
                match diesel::update(schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id)))
                    .set((name_update, url_key_update))
                    .get_result::<Workspace>(&mut conn)
                {
                    Ok(workspace) => workspace,
                    Err(e) => {
                        // 检查是否是唯一性约束违反错误
                        if e.to_string().contains("url_key") {
                            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                                field: Some("url_key".to_string()),
                                code: "DUPLICATE".to_string(),
                                message: "Workspace with this url_key already exists".to_string(),
                            }]);
                            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                        } else {
                            let response = ApiResponse::<()>::internal_error("Failed to update workspace");
                            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                        }
                    }
                }
            }
        } else if let Some(logo_url_update) = logo_url_update {
            // 更新 name 和 logo_url
            match diesel::update(schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id)))
                .set((name_update, logo_url_update))
                .get_result::<Workspace>(&mut conn)
            {
                Ok(workspace) => workspace,
                Err(_) => {
                    let response = ApiResponse::<()>::internal_error("Failed to update workspace");
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                }
            }
        } else {
            // 只更新 name
            match diesel::update(schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id)))
                .set(name_update)
                .get_result::<Workspace>(&mut conn)
            {
                Ok(workspace) => workspace,
                Err(_) => {
                    let response = ApiResponse::<()>::internal_error("Failed to update workspace");
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                }
            }
        }
    } else if let Some(url_key_update) = url_key_update {
        if let Some(logo_url_update) = logo_url_update {
            // 更新 url_key 和 logo_url
            match diesel::update(schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id)))
                .set((url_key_update, logo_url_update))
                .get_result::<Workspace>(&mut conn)
            {
                Ok(workspace) => workspace,
                Err(e) => {
                    // 检查是否是唯一性约束违反错误
                    if e.to_string().contains("url_key") {
                        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                            field: Some("url_key".to_string()),
                            code: "DUPLICATE".to_string(),
                            message: "Workspace with this url_key already exists".to_string(),
                        }]);
                        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                    } else {
                        let response = ApiResponse::<()>::internal_error("Failed to update workspace");
                        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                    }
                }
            }
        } else {
            // 只更新 url_key
            match diesel::update(schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id)))
                .set(url_key_update)
                .get_result::<Workspace>(&mut conn)
            {
                Ok(workspace) => workspace,
                Err(e) => {
                    // 检查是否是唯一性约束违反错误
                    if e.to_string().contains("url_key") {
                        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                            field: Some("url_key".to_string()),
                            code: "DUPLICATE".to_string(),
                            message: "Workspace with this url_key already exists".to_string(),
                        }]);
                        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                    } else {
                        let response = ApiResponse::<()>::internal_error("Failed to update workspace");
                        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                    }
                }
            }
        }
    } else if let Some(logo_url_update) = logo_url_update {
        // 只更新 logo_url
        match diesel::update(schema::workspaces::table.filter(schema::workspaces::id.eq(workspace_id)))
            .set(logo_url_update)
            .get_result::<Workspace>(&mut conn)
        {
            Ok(workspace) => workspace,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Failed to update workspace");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    } else {
        // 没有要更新的字段，这种情况已经被前面处理过了，但为了完整性保留
        existing_workspace
    };

    let response = ApiResponse::success(
        Some(updated_workspace),
        "Workspace updated successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

// 删除工作空间
pub async fn delete_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(workspace_id): Path<Uuid>,
) -> impl IntoResponse {
    let _user_id = auth_info.user.id;

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查工作空间是否存在
    use crate::schema::workspaces::dsl::*;
    let existing_workspace = match workspaces
        .filter(id.eq(workspace_id))
        .select(Workspace::as_select())
        .first::<Workspace>(&mut conn)
    {
        Ok(workspace) => workspace,
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Workspace not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    // 检查用户是否是工作空间的所有者
    if existing_workspace.id != workspace_id {
        let response = ApiResponse::<()>::forbidden("Only workspace owner can delete workspace");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 删除工作空间
    match diesel::delete(workspaces.filter(id.eq(workspace_id))).execute(&mut conn) {
        Ok(_) => {
            // 清除所有用户的 current_workspace_id
            use crate::schema::users::dsl::*;
            let _ = diesel::update(
                users.filter(current_workspace_id.eq(workspace_id))
            )
            .set(current_workspace_id.eq(None::<Uuid>))
            .execute(&mut conn);

            let response = ApiResponse::<()>::success((), "Workspace deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        },
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to delete workspace");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}