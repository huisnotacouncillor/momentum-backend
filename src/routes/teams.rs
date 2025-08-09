use crate::db::{DbPool, models::*};
use crate::middleware::auth::{AuthConfig, AuthService};
use crate::schema;
use axum::TypedHeader;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use headers::Authorization;
use headers::authorization::Bearer;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

// 请求体定义
#[derive(Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub team_key: String,
}

#[derive(Deserialize)]
pub struct AddTeamMemberRequest {
    pub user_id: Uuid,
    pub role: String,
}

#[derive(Deserialize)]
pub struct UpdateTeamMemberRequest {
    pub role: String,
}

/// 创建团队
pub async fn create_team(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateTeamRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 获取用户当前工作空间
    let user = match schema::users::table
        .filter(schema::users::id.eq(claims.sub))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .optional()
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            let response = ApiResponse::<()>::unauthorized("User not found");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let workspace_id = match user.current_workspace_id {
        Some(id) => id,
        None => {
            let response = ApiResponse::<()>::forbidden("No active workspace");
            return (StatusCode::FORBIDDEN, Json(response)).into_response();
        }
    };

    // 验证团队名称和键不为空
    if payload.name.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("name".to_string()),
            code: "REQUIRED".to_string(),
            message: "Team name is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    if payload.team_key.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("team_key".to_string()),
            code: "REQUIRED".to_string(),
            message: "Team key is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 检查团队键在工作空间中是否唯一
    let key_exists = match schema::teams::table
        .filter(schema::teams::workspace_id.eq(workspace_id))
        .filter(schema::teams::team_key.eq(payload.team_key.trim().to_uppercase()))
        .select(schema::teams::id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if key_exists.is_some() {
        let response = ApiResponse::<()>::conflict(
            "Team key already exists in this workspace",
            Some("team_key".to_string()),
            "TEAM_KEY_EXISTS",
        );
        return (StatusCode::CONFLICT, Json(response)).into_response();
    }

    // 创建新团队
    let new_team = NewTeam {
        workspace_id,
        name: payload.name.trim().to_string(),
        team_key: payload.team_key.trim().to_uppercase(),
    };

    let team = match diesel::insert_into(schema::teams::table)
        .values(&new_team)
        .get_result::<Team>(&mut conn)
    {
        Ok(team) => team,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to create team");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 将创建者添加为团队成员，角色为 "admin"
    let new_team_member = NewTeamMember {
        user_id: claims.sub,
        team_id: team.id,
        role: "admin".to_string(),
    };

    if diesel::insert_into(schema::team_members::table)
        .values(&new_team_member)
        .execute(&mut conn)
        .is_err()
    {
        let response = ApiResponse::<()>::internal_error("Failed to add team member");
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
    }

    // 获取团队成员信息
    let members = match get_team_members(&mut conn, team.id) {
        Ok(members) => members,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to fetch team members");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let team_detail = team::TeamDetailResponse {
        id: team.id,
        name: team.name,
        team_key: team.team_key,
        workspace_id: team.workspace_id,
        members,
        created_at: team.created_at,
        updated_at: team.updated_at,
    };

    let response = ApiResponse::created(team_detail, "Team created successfully");
    (StatusCode::CREATED, Json(response)).into_response()
}

/// 获取工作空间中的所有团队
pub async fn get_teams(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 获取用户当前工作空间
    let user = match schema::users::table
        .filter(schema::users::id.eq(claims.sub))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .optional()
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            let response = ApiResponse::<()>::unauthorized("User not found");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let workspace_id = match user.current_workspace_id {
        Some(id) => id,
        None => {
            let response = ApiResponse::<()>::forbidden("No active workspace");
            return (StatusCode::FORBIDDEN, Json(response)).into_response();
        }
    };

    // 获取工作空间中的所有团队
    let teams = match schema::teams::table
        .filter(schema::teams::workspace_id.eq(workspace_id))
        .select(Team::as_select())
        .load::<Team>(&mut conn)
    {
        Ok(teams) => teams,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 获取每个团队的成员信息
    let mut teams_with_members: Vec<team::TeamWithMembers> = Vec::new();

    for team in teams {
        // 获取团队成员信息
        let members = match get_team_members(&mut conn, team.id) {
            Ok(members) => members,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Failed to fetch team members");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        };

        teams_with_members.push(team::TeamWithMembers {
            id: team.id,
            name: team.name,
            team_key: team.team_key,
            members,
        });
    }

    let response = ApiResponse::success(teams_with_members, "Teams retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

/// 获取团队详情
pub async fn get_team(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 验证用户是否有权限访问该团队
    let team = match schema::teams::table
        .filter(schema::teams::id.eq(team_id))
        .select(Team::as_select())
        .first::<Team>(&mut conn)
        .optional()
    {
        Ok(Some(team)) => team,
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证用户是否是团队成员
    let is_member = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .select(schema::team_members::user_id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result.is_some(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if !is_member {
        let response = ApiResponse::<()>::forbidden("You are not a member of this team");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 获取团队成员信息
    let members = match get_team_members(&mut conn, team.id) {
        Ok(members) => members,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to fetch team members");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let team_detail = team::TeamDetailResponse {
        id: team.id,
        name: team.name,
        team_key: team.team_key,
        workspace_id: team.workspace_id,
        members,
        created_at: team.created_at,
        updated_at: team.updated_at,
    };

    let response = ApiResponse::success(team_detail, "Team retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

/// 更新团队信息
pub async fn update_team(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<CreateTeamRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 验证用户是否有权限更新该团队（必须是管理员）
    let user_role = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .select(schema::team_members::role)
        .first::<String>(&mut conn)
        .optional()
    {
        Ok(Some(role)) => role,
        Ok(None) => {
            let response = ApiResponse::<()>::forbidden("You are not a member of this team");
            return (StatusCode::FORBIDDEN, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if user_role != "admin" {
        let response = ApiResponse::<()>::forbidden("Only team admins can update team information");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 验证团队名称和键不为空
    if payload.name.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("name".to_string()),
            code: "REQUIRED".to_string(),
            message: "Team name is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    if payload.team_key.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("team_key".to_string()),
            code: "REQUIRED".to_string(),
            message: "Team key is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 获取团队信息
    let team = match schema::teams::table
        .filter(schema::teams::id.eq(team_id))
        .select(Team::as_select())
        .first::<Team>(&mut conn)
        .optional()
    {
        Ok(Some(team)) => team,
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查团队键在工作空间中是否唯一（排除当前团队）
    let key_exists = match schema::teams::table
        .filter(schema::teams::workspace_id.eq(team.workspace_id))
        .filter(schema::teams::team_key.eq(payload.team_key.trim().to_uppercase()))
        .filter(schema::teams::id.ne(team_id))
        .select(schema::teams::id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if key_exists.is_some() {
        let response = ApiResponse::<()>::conflict(
            "Team key already exists in this workspace",
            Some("team_key".to_string()),
            "TEAM_KEY_EXISTS",
        );
        return (StatusCode::CONFLICT, Json(response)).into_response();
    }

    // 更新团队信息
    let updated_team = match diesel::update(schema::teams::table.filter(schema::teams::id.eq(team_id)))
        .set((
            schema::teams::name.eq(payload.name.trim()),
            schema::teams::team_key.eq(payload.team_key.trim().to_uppercase()),
            schema::teams::updated_at.eq(chrono::Utc::now()),
        ))
        .get_result::<Team>(&mut conn)
    {
        Ok(team) => team,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update team");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 获取团队成员信息
    let members = match get_team_members(&mut conn, updated_team.id) {
        Ok(members) => members,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to fetch team members");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let team_detail = team::TeamDetailResponse {
        id: updated_team.id,
        name: updated_team.name,
        team_key: updated_team.team_key,
        workspace_id: updated_team.workspace_id,
        members,
        created_at: updated_team.created_at,
        updated_at: updated_team.updated_at,
    };

    let response = ApiResponse::success(team_detail, "Team updated successfully");
    (StatusCode::OK, Json(response)).into_response()
}

/// 删除团队
pub async fn delete_team(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 验证用户是否有权限删除该团队（必须是管理员）
    let user_role = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .select(schema::team_members::role)
        .first::<String>(&mut conn)
        .optional()
    {
        Ok(Some(role)) => role,
        Ok(None) => {
            let response = ApiResponse::<()>::forbidden("You are not a member of this team");
            return (StatusCode::FORBIDDEN, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if user_role != "admin" {
        let response = ApiResponse::<()>::forbidden("Only team admins can delete the team");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 检查团队是否存在
    let team_exists = match schema::teams::table
        .filter(schema::teams::id.eq(team_id))
        .select(schema::teams::id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result.is_some(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if !team_exists {
        let response = ApiResponse::<()>::not_found("Team not found");
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    // 删除团队（这将级联删除团队成员）
    match diesel::delete(schema::teams::table.filter(schema::teams::id.eq(team_id)))
        .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Team deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
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
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<AddTeamMemberRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 验证用户是否有权限添加成员（必须是管理员）
    let user_role = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .select(schema::team_members::role)
        .first::<String>(&mut conn)
        .optional()
    {
        Ok(Some(role)) => role,
        Ok(None) => {
            let response = ApiResponse::<()>::forbidden("You are not a member of this team");
            return (StatusCode::FORBIDDEN, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if user_role != "admin" {
        let response = ApiResponse::<()>::forbidden("Only team admins can add members");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 检查要添加的用户是否存在
    let user_exists = match schema::users::table
        .filter(schema::users::id.eq(payload.user_id))
        .select(schema::users::id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result.is_some(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if !user_exists {
        let response = ApiResponse::<()>::not_found("User not found");
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    // 检查用户是否已经是团队成员
    let member_exists = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(payload.user_id))
        .select(schema::team_members::user_id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result.is_some(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if member_exists {
        let response = ApiResponse::<()>::conflict(
            "User is already a member of this team",
            Some("user_id".to_string()),
            "USER_ALREADY_MEMBER",
        );
        return (StatusCode::CONFLICT, Json(response)).into_response();
    }

    // 添加团队成员
    let new_team_member = NewTeamMember {
        user_id: payload.user_id,
        team_id,
        role: payload.role,
    };

    match diesel::insert_into(schema::team_members::table)
        .values(&new_team_member)
        .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Member added successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to add member");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 获取团队成员列表
pub async fn get_team_members_list(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 验证用户是否是团队成员
    let is_member = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .select(schema::team_members::user_id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result.is_some(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if !is_member {
        let response = ApiResponse::<()>::forbidden("You are not a member of this team");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 获取团队成员信息
    let members = match get_team_members(&mut conn, team_id) {
        Ok(members) => members,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to fetch team members");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(members, "Team members retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

/// 更新团队成员角色
pub async fn update_team_member(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateTeamMemberRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 验证用户是否有权限更新成员（必须是管理员，且不能修改自己的角色）
    if claims.sub == user_id {
        let response = ApiResponse::<()>::forbidden("You cannot update your own role");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    let user_role = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .select(schema::team_members::role)
        .first::<String>(&mut conn)
        .optional()
    {
        Ok(Some(role)) => role,
        Ok(None) => {
            let response = ApiResponse::<()>::forbidden("You are not a member of this team");
            return (StatusCode::FORBIDDEN, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if user_role != "admin" {
        let response = ApiResponse::<()>::forbidden("Only team admins can update member roles");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 检查要更新的用户是否是团队成员
    let member_exists = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(user_id))
        .select(schema::team_members::user_id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result.is_some(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if !member_exists {
        let response = ApiResponse::<()>::not_found("User is not a member of this team");
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    // 更新团队成员角色
    match diesel::update(
        schema::team_members::table
            .filter(schema::team_members::team_id.eq(team_id))
            .filter(schema::team_members::user_id.eq(user_id)),
    )
    .set(schema::team_members::role.eq(&payload.role))
    .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Member role updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update member role");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 删除团队成员
pub async fn remove_team_member(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 验证用户是否有权限删除成员（必须是管理员，且不能删除自己）
    if claims.sub == user_id {
        let response = ApiResponse::<()>::forbidden("You cannot remove yourself from the team");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    let user_role = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .select(schema::team_members::role)
        .first::<String>(&mut conn)
        .optional()
    {
        Ok(Some(role)) => role,
        Ok(None) => {
            let response = ApiResponse::<()>::forbidden("You are not a member of this team");
            return (StatusCode::FORBIDDEN, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if user_role != "admin" {
        let response = ApiResponse::<()>::forbidden("Only team admins can remove members");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 检查要删除的用户是否是团队成员
    let member_exists = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .filter(schema::team_members::user_id.eq(user_id))
        .select(schema::team_members::user_id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result.is_some(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if !member_exists {
        let response = ApiResponse::<()>::not_found("User is not a member of this team");
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    // 删除团队成员
    match diesel::delete(
        schema::team_members::table
            .filter(schema::team_members::team_id.eq(team_id))
            .filter(schema::team_members::user_id.eq(user_id)),
    )
    .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Member removed successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to remove member");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 获取用户的团队列表（包括角色信息）
pub async fn get_user_teams(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 获取用户所属的所有团队及角色
    let teams_with_roles = match schema::team_members::table
        .inner_join(schema::teams::table)
        .filter(schema::team_members::user_id.eq(claims.sub))
        .select((Team::as_select(), schema::team_members::role))
        .load::<(Team, String)>(&mut conn)
    {
        Ok(results) => results,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let team_infos: Vec<TeamInfo> = teams_with_roles
        .into_iter()
        .map(|(team, role)| TeamInfo {
            id: team.id,
            name: team.name,
            team_key: team.team_key,
            role,
        })
        .collect();

    let response = ApiResponse::success(team_infos, "User teams retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

// 辅助函数：获取团队成员列表
fn get_team_members(conn: &mut PgConnection, team_id: Uuid) -> Result<Vec<team::TeamMemberInfo>, diesel::result::Error> {
    let members = schema::team_members::table
        .inner_join(schema::users::table)
        .filter(schema::team_members::team_id.eq(team_id))
        .select((User::as_select(), TeamMember::as_select()))
        .load::<(User, TeamMember)>(conn)?
        .into_iter()
        .map(|(user, team_member)| {
            let user_info = UserBasicInfo {
                id: user.id,
                name: user.name,
                username: user.username,
                email: user.email,
                avatar_url: user.avatar_url,
            };

            team::TeamMemberInfo {
                user: user_info,
                role: team_member.role,
                joined_at: team_member.joined_at,
            }
        })
        .collect();

    Ok(members)
}