use crate::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json
};
use diesel::prelude::*;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::models::*;
use crate::middleware::auth::AuthUserInfo;

// 定义响应数据结构
#[derive(Serialize)]
pub struct WorkspaceMemberInfo {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub role: WorkspaceMemberRole,
    pub user: UserBasicInfo,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// 获取工作区成员列表
pub async fn get_workspace_members(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    use crate::schema::workspace_members::dsl::*;
    use crate::schema::users::dsl as user_dsl;

    let members = match workspace_members
        .filter(workspace_id.eq(current_workspace_id))
        .inner_join(user_dsl::users.on(user_dsl::id.eq(user_id)))
        .select((WorkspaceMember::as_select(), User::as_select()))
        .load::<(WorkspaceMember, User)>(&mut conn)
    {
        Ok(results) => results
            .into_iter()
            .map(|(member, user)| WorkspaceMemberInfo {
                user_id: member.user_id,
                workspace_id: member.workspace_id,
                role: member.role,
                user: UserBasicInfo {
                    id: user.id,
                    name: user.name,
                    username: user.username,
                    email: user.email,
                    avatar_url: user.avatar_url,
                },
                created_at: member.created_at,
                updated_at: member.updated_at,
            })
            .collect::<Vec<WorkspaceMemberInfo>>(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve workspace members");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(members),
        "Workspace members retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

// 获取当前工作区成员列表
pub async fn get_current_workspace_members(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    use crate::schema::workspace_members::dsl::*;
    use crate::schema::users::dsl as user_dsl;

    let members = match workspace_members
        .filter(workspace_id.eq(current_workspace_id))
        .inner_join(user_dsl::users.on(user_dsl::id.eq(user_id)))
        .select((WorkspaceMember::as_select(), User::as_select()))
        .load::<(WorkspaceMember, User)>(&mut conn)
    {
        Ok(results) => results
            .into_iter()
            .map(|(member, user)| WorkspaceMemberInfo {
                user_id: member.user_id,
                workspace_id: member.workspace_id,
                role: member.role,
                user: UserBasicInfo {
                    id: user.id,
                    name: user.name,
                    username: user.username,
                    email: user.email,
                    avatar_url: user.avatar_url,
                },
                created_at: member.created_at,
                updated_at: member.updated_at,
            })
            .collect::<Vec<WorkspaceMemberInfo>>(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve workspace members");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(members),
        "Current workspace members retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}