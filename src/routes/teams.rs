use crate::db::models::{ApiResponse, ErrorDetail, TeamInfo, TeamMemberInfo};
use crate::db::{DbPool, models::*};
use crate::middleware::auth::AuthUserInfo;
use crate::services::context::RequestContext;
use crate::services::{team_members_service::TeamMembersService, teams_service::TeamsService};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

// 请求体定义
#[derive(Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
}

#[derive(Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub team_key: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: Option<bool>,
}

#[derive(Deserialize, Clone, Copy)]
pub enum TeamRole {
    Admin,
    Member,
}

#[derive(Deserialize)]
pub struct AddTeamMemberRequest {
    pub user_id: Uuid,
    pub role: TeamRole,
}

#[derive(Deserialize)]
pub struct UpdateTeamMemberRequest {
    pub role: TeamRole,
}

/// 创建团队
pub async fn create_team(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateTeamRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap(),
        idempotency_key: None,
    };

    let req = CreateTeamRequest {
        name: payload.name,
        team_key: payload.team_key,
        description: payload.description,
        icon_url: payload.icon_url,
        is_private: payload.is_private,
    };

    match TeamsService::create(&mut conn, &ctx, &req) {
        Ok(team) => {
            let response = ApiResponse::success(Some(team), "Team created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => match e {
            crate::error::AppError::Validation { message } => {
                let response = ApiResponse::<()>::bad_request(&message);
                (StatusCode::BAD_REQUEST, Json(response)).into_response()
            }
            _ => {
                let response = ApiResponse::<()>::internal_error("Failed to create team");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
        },
    }
}

/// 获取团队列表
pub async fn get_teams(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap(),
        idempotency_key: None,
    };

    match TeamsService::list(&mut conn, &ctx) {
        Ok(list) => {
            let response = ApiResponse::success(Some(list), "Teams retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve teams");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 获取指定团队
pub async fn get_team(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap(),
        idempotency_key: None,
    };

    match TeamsService::get(&mut conn, &ctx, team_id) {
        Ok(team) => {
            let response = ApiResponse::success(Some(team), "Team retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
    }
}

/// 更新团队
pub async fn update_team(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<UpdateTeamRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap(),
        idempotency_key: None,
    };

    let req = UpdateTeamRequest {
        name: payload.name,
        team_key: payload.team_key,
        description: payload.description,
        icon_url: payload.icon_url,
        is_private: payload.is_private,
    };

    match TeamsService::update(&mut conn, &ctx, team_id, &req) {
        Ok(updated_team) => {
            let response = ApiResponse::success(Some(updated_team), "Team updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => match e {
            crate::error::AppError::Validation { message } => {
                let response = ApiResponse::<()>::bad_request(&message);
                (StatusCode::BAD_REQUEST, Json(response)).into_response()
            }
            crate::error::AppError::NotFound { .. } => {
                let response = ApiResponse::<()>::not_found("Team not found");
                (StatusCode::NOT_FOUND, Json(response)).into_response()
            }
            _ => {
                let response = ApiResponse::<()>::internal_error("Failed to update team");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
        },
    }
}

/// 删除团队
pub async fn delete_team(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap(),
        idempotency_key: None,
    };

    match TeamsService::delete(&mut conn, &ctx, team_id) {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Team deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::error::AppError::NotFound { .. }) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to delete team");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 添加团队成员
pub async fn add_team_member(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<AddTeamMemberRequest>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: current_workspace_id,
        idempotency_key: None,
    };
    let role_str = match payload.role {
        TeamRole::Admin => "admin",
        TeamRole::Member => "member",
    };
    match TeamMembersService::add(&mut conn, &ctx, team_id, payload.user_id, role_str) {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Team member added successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(crate::error::AppError::Validation { message }) => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: Some("user_id".to_string()),
                code: "INVALID".to_string(),
                message,
            }]);
            (StatusCode::BAD_REQUEST, Json(response)).into_response()
        }
        Err(crate::error::AppError::NotFound { .. }) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to add team member");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 获取团队成员列表
pub async fn get_team_members_list(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap(),
        idempotency_key: None,
    };

    let members = match crate::services::team_members_service::TeamMembersService::list(
        &mut conn, &ctx, team_id,
    ) {
        Ok(results) => results,
        Err(crate::error::AppError::NotFound { .. }) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve team members");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let members = members
        .into_iter()
        .map(|(member, user)| TeamMemberInfo {
            user: crate::db::models::auth::UserBasicInfo {
                id: user.id,
                name: user.name,
                username: user.username,
                email: user.email,
                avatar_url: user.avatar_url,
            },
            role: member.role,
            joined_at: member.joined_at,
        })
        .collect::<Vec<TeamMemberInfo>>();

    let response = ApiResponse::success(Some(members), "Team members retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

/// 更新团队成员
pub async fn update_team_member(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path((team_id, member_user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateTeamMemberRequest>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: current_workspace_id,
        idempotency_key: None,
    };
    let role_str = match payload.role {
        TeamRole::Admin => "admin",
        TeamRole::Member => "member",
    };
    match TeamMembersService::update(&mut conn, &ctx, team_id, member_user_id, role_str) {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Team member updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::error::AppError::NotFound { .. }) => {
            let response = ApiResponse::<()>::not_found("Team or member not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update team member");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 移除团队成员
pub async fn remove_team_member(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path((team_id, member_user_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: current_workspace_id,
        idempotency_key: None,
    };
    match TeamMembersService::remove(&mut conn, &ctx, team_id, member_user_id) {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Team member removed successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::error::AppError::NotFound { .. }) => {
            let response = ApiResponse::<()>::not_found("Team or member not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to remove team member");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 获取用户所属的团队列表
pub async fn get_user_teams(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let current_user_id = auth_info.user.id;
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    use crate::schema::team_members::dsl::*;
    use crate::schema::teams::dsl as team_dsl;

    let user_teams = match team_members
        .filter(user_id.eq(current_user_id))
        .inner_join(team_dsl::teams.on(team_dsl::id.eq(team_id)))
        .filter(team_dsl::workspace_id.eq(current_workspace_id))
        .select((TeamMember::as_select(), Team::as_select()))
        .load::<(TeamMember, Team)>(&mut conn)
    {
        Ok(results) => results
            .into_iter()
            .map(|(member, team)| TeamInfo {
                id: team.id,
                name: team.name,
                team_key: team.team_key,
                description: team.description,
                icon_url: team.icon_url,
                is_private: team.is_private,
                role: member.role,
            })
            .collect::<Vec<TeamInfo>>(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve user teams");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(Some(user_teams), "User teams retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}
