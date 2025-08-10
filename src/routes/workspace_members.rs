use crate::AppState;
use axum::{
    extract::{Path, State, TypedHeader},
    http::StatusCode,
    response::IntoResponse,
    Json
};
use diesel::prelude::*;
use headers::{Authorization, authorization::Bearer};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::cache::get_user_current_workspace_id_cached;
use crate::db::models::*;
use crate::middleware::auth::{AuthService, AuthConfig};
use crate::schema;

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

    // 验证用户是否有权访问该工作区
    let has_access = match schema::workspace_members::table
        .filter(schema::workspace_members::workspace_id.eq(workspace_id))
        .filter(schema::workspace_members::user_id.eq(claims.sub))
        .select(schema::workspace_members::user_id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result.is_some(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if !has_access {
        let response = ApiResponse::<()>::forbidden("You don't have access to this workspace");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 查询工作区成员列表
    let members_result: Result<Vec<(WorkspaceMember, User)>, diesel::result::Error> = 
        schema::workspace_members::table
            .filter(schema::workspace_members::workspace_id.eq(workspace_id))
            .inner_join(
                schema::users::table.on(
                    schema::workspace_members::user_id.eq(schema::users::id)
                )
            )
            .select((
                WorkspaceMember::as_select(),
                User::as_select()
            ))
            .load::<(WorkspaceMember, User)>(&mut conn);

    match members_result {
        Ok(members) => {
            let member_infos: Vec<WorkspaceMemberInfo> = members
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
                .collect();

            let response = ApiResponse::success(
                member_infos, 
                "Workspace members retrieved successfully"
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve workspace members");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

// 获取当前工作区的成员列表
pub async fn get_current_workspace_members(
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

    // 从Redis获取当前用户的workspace_id
    let workspace_id = match get_user_current_workspace_id_cached(&state.redis, &state.db, claims.sub).await {
        Some(id) => id,
        None => {
            let response = ApiResponse::<()>::error(
                400, 
                "No current workspace found", 
                vec![ErrorDetail {
                    field: None,
                    code: "NO_WORKSPACE".to_string(),
                    message: "No current workspace found for user".to_string(),
                }]
            );
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 查询工作区成员列表
    let members_result: Result<Vec<(WorkspaceMember, User)>, diesel::result::Error> = 
        schema::workspace_members::table
            .filter(schema::workspace_members::workspace_id.eq(workspace_id))
            .inner_join(
                schema::users::table.on(
                    schema::workspace_members::user_id.eq(schema::users::id)
                )
            )
            .select((
                WorkspaceMember::as_select(),
                User::as_select()
            ))
            .load::<(WorkspaceMember, User)>(&mut conn);

    match members_result {
        Ok(members) => {
            let member_infos: Vec<WorkspaceMemberInfo> = members
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
                .collect();

            let response = ApiResponse::success(
                member_infos, 
                "Workspace members retrieved successfully"
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve workspace members");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}