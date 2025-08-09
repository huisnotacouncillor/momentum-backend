use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use bcrypt::{DEFAULT_COST, hash, verify};
use diesel::prelude::*;

use crate::{
    db::{DbPool, models::*},
    middleware::auth::{AuthConfig, AuthService},
    schema,
};
use axum::TypedHeader;
use headers::Authorization;
use headers::authorization::Bearer;
use std::{collections::HashMap, sync::Arc};

pub async fn register(
    State(pool): State<Arc<DbPool>>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查邮箱是否已存在
    let existing_user = match schema::users::table
        .filter(schema::users::email.eq(&payload.email))
        .select(User::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(user) => user,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if existing_user.is_some() {
        let response = ApiResponse::<()>::conflict(
            "Email address already exists",
            Some("email".to_string()),
            error_codes::USER_EMAIL_EXISTS,
        );
        return (StatusCode::CONFLICT, Json(response)).into_response();
    }

    // 检查用户名是否已存在
    let existing_username = match schema::users::table
        .filter(schema::users::username.eq(&payload.username))
        .first::<User>(&mut conn)
        .optional()
    {
        Ok(user) => user,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if existing_username.is_some() {
        let response = ApiResponse::<()>::conflict(
            "Username already exists",
            Some("username".to_string()),
            error_codes::USER_USERNAME_EXISTS,
        );
        return (StatusCode::CONFLICT, Json(response)).into_response();
    }

    // 使用事务来确保所有操作要么全部成功，要么全部失败
    let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // 创建新用户
        let new_user = NewUser {
            email: payload.email.clone(),
            username: payload.username.clone(),
            name: payload.name.clone(),
            avatar_url: None,
        };

        let user: User = diesel::insert_into(schema::users::table)
            .values(&new_user)
            .get_result(conn)?;

        // 哈希密码
        let password_hash = hash(payload.password.as_bytes(), DEFAULT_COST)
            .map_err(|_| diesel::result::Error::RollbackTransaction)?;

        // 创建用户认证记录
        let new_credential = NewUserCredential {
            user_id: user.id,
            credential_type: "password".to_string(),
            credential_hash: Some(password_hash),
            oauth_provider_id: None,
            oauth_user_id: None,
            is_primary: true,
        };

        diesel::insert_into(schema::user_credentials::table)
            .values(&new_credential)
            .execute(conn)?;

        // 创建默认工作空间
        let workspace_name = format!("{}'s Workspace", payload.name);
        let workspace_url_key = format!("{}-workspace", payload.username.to_lowercase());

        let new_workspace = NewWorkspace {
            name: workspace_name,
            url_key: workspace_url_key,
        };

        let workspace: Workspace = diesel::insert_into(schema::workspaces::table)
            .values(&new_workspace)
            .get_result(conn)?;

        // 创建默认团队
        let team_name = "Default Team".to_string();
        let team_key = "DEF".to_string();

        let new_team = NewTeam {
            workspace_id: workspace.id,
            name: team_name,
            team_key,
        };

        let team: Team = diesel::insert_into(schema::teams::table)
            .values(&new_team)
            .get_result(conn)?;

        // 将用户添加为团队成员，角色为 "admin"
        let new_team_member = NewTeamMember {
            user_id: user.id,
            team_id: team.id,
            role: "admin".to_string(),
        };

        diesel::insert_into(schema::team_members::table)
            .values(&new_team_member)
            .execute(conn)?;

        // 设置用户的当前workspace为新创建的默认workspace
        diesel::update(schema::users::table)
            .filter(schema::users::id.eq(user.id))
            .set(schema::users::current_workspace_id.eq(Some(workspace.id)))
            .execute(conn)?;

        Ok(user)
    });

    let user = match result {
        Ok(user) => user,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to create user account");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 生成token
    let auth_service = AuthService::new(AuthConfig::default());
    let auth_user = AuthUser {
        id: user.id,
        email: user.email,
        username: user.username,
        name: user.name,
        avatar_url: user.avatar_url,
    };

    let access_token = match auth_service.generate_access_token(&auth_user) {
        Ok(token) => token,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to generate access token");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let refresh_token = match auth_service.generate_refresh_token(user.id) {
        Ok(token) => token,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to generate refresh token");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let login_data = LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        user: auth_user,
    };

    let response = ApiResponse::created(login_data, "User registered successfully");
    (StatusCode::CREATED, Json(response)).into_response()
}

pub async fn login(
    State(pool): State<Arc<DbPool>>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 查找用户
    let user: User = match schema::users::table
        .filter(schema::users::email.eq(&payload.email))
        .filter(schema::users::is_active.eq(true))
        .select(User::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            let response = ApiResponse::<()>::unauthorized("Invalid email or password");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 查找用户认证信息
    let credential: UserCredential = match schema::user_credentials::table
        .filter(schema::user_credentials::user_id.eq(user.id))
        .filter(schema::user_credentials::credential_type.eq("password"))
        .filter(schema::user_credentials::is_primary.eq(true))
        .select(UserCredential::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(Some(cred)) => cred,
        Ok(None) => {
            let response = ApiResponse::<()>::unauthorized("Invalid email or password");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证密码
    if let Some(hash) = credential.credential_hash {
        let is_valid = match verify(payload.password.as_bytes(), &hash) {
            Ok(valid) => valid,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Password verification failed");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        };

        if !is_valid {
            let response = ApiResponse::<()>::unauthorized("Invalid email or password");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    } else {
        let response = ApiResponse::<()>::unauthorized("Invalid email or password");
        return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
    }

    // 生成token
    let auth_service = AuthService::new(AuthConfig::default());
    let auth_user = AuthUser {
        id: user.id,
        email: user.email,
        username: user.username,
        name: user.name,
        avatar_url: user.avatar_url,
    };

    let access_token = match auth_service.generate_access_token(&auth_user) {
        Ok(token) => token,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to generate access token");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let refresh_token = match auth_service.generate_refresh_token(user.id) {
        Ok(token) => token,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to generate refresh token");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let login_data = LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        user: auth_user,
    };

    let response = ApiResponse::success(login_data, "Login successful");
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn refresh_token(
    State(pool): State<Arc<DbPool>>,
    Json(payload): Json<RefreshTokenRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证refresh token
    let auth_service = AuthService::new(AuthConfig::default());
    let refresh_claims = match auth_service.verify_refresh_token(&payload.refresh_token) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired refresh token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 获取用户信息
    let user: User = match schema::users::table
        .filter(schema::users::id.eq(refresh_claims.sub))
        .filter(schema::users::is_active.eq(true))
        .select(User::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            let response = ApiResponse::<()>::unauthorized("User not found or inactive");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 生成新的token
    let auth_user = AuthUser {
        id: user.id,
        email: user.email,
        username: user.username,
        name: user.name,
        avatar_url: user.avatar_url,
    };

    let new_access_token = match auth_service.generate_access_token(&auth_user) {
        Ok(token) => token,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to generate access token");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let new_refresh_token = match auth_service.generate_refresh_token(user.id) {
        Ok(token) => token,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to generate refresh token");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let refresh_data = LoginResponse {
        access_token: new_access_token,
        refresh_token: new_refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        user: auth_user,
    };

    let response = ApiResponse::success(refresh_data, "Token refreshed successfully");
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn logout(State(_pool): State<Arc<DbPool>>) -> impl IntoResponse {
    // 简化版本，暂时不处理会话
    let response = ApiResponse::<()>::ok("Logout successful");
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn get_profile(
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

    // 查找用户
    let user: User = match schema::users::table
        .filter(schema::users::id.eq(claims.sub))
        .filter(schema::users::is_active.eq(true))
        .select(User::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            let response = ApiResponse::<()>::unauthorized("User not found or inactive");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 查询用户所属的团队和对应的工作空间
    let teams_with_workspaces: Vec<(Team, Workspace, String)> = match schema::team_members::table
        .inner_join(schema::teams::table.on(schema::teams::id.eq(schema::team_members::team_id)))
        .inner_join(
            schema::workspaces::table.on(schema::workspaces::id.eq(schema::teams::workspace_id)),
        )
        .filter(schema::team_members::user_id.eq(user.id))
        .select((
            Team::as_select(),
            Workspace::as_select(),
            schema::team_members::role,
        ))
        .load(&mut conn)
    {
        Ok(data) => data,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 提取唯一的工作空间
    let mut workspace_map = HashMap::new();
    let mut teams = Vec::new();

    for (team, workspace, role) in teams_with_workspaces {
        // 添加工作空间到映射（自动去重）
        workspace_map.insert(
            workspace.id,
            WorkspaceInfo {
                id: workspace.id,
                name: workspace.name,
                url_key: workspace.url_key,
            },
        );

        // 添加团队信息
        teams.push(TeamInfo {
            id: team.id,
            name: team.name,
            team_key: team.team_key,
            role,
        });
    }

    // 转换工作空间映射为向量
    let workspaces: Vec<WorkspaceInfo> = workspace_map.into_values().collect();

    let user_profile = UserProfile {
        id: user.id,
        email: user.email,
        username: user.username,
        name: user.name,
        avatar_url: user.avatar_url,
        current_workspace_id: user.current_workspace_id,
        workspaces,
        teams,
    };

    let response = ApiResponse::success(user_profile, "Profile retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn switch_workspace(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<SwitchWorkspaceRequest>,
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

    // 获取用户当前信息（包括当前workspace_id）
    let current_user = match schema::users::table
        .filter(schema::users::id.eq(claims.sub))
        .filter(schema::users::is_active.eq(true))
        .select(User::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("User not found or inactive");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证目标workspace是否存在并获取其信息
    let target_workspace = match schema::workspaces::table
        .filter(schema::workspaces::id.eq(payload.workspace_id))
        .select(Workspace::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Workspace not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证用户是否有权限访问指定的workspace，并获取用户在该workspace的角色信息
    let user_teams_in_workspace: Vec<(Team, String)> = match schema::team_members::table
        .inner_join(schema::teams::table.on(schema::teams::id.eq(schema::team_members::team_id)))
        .filter(schema::team_members::user_id.eq(claims.sub))
        .filter(schema::teams::workspace_id.eq(payload.workspace_id))
        .select((Team::as_select(), schema::team_members::role))
        .load(&mut conn)
    {
        Ok(teams) => teams,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if user_teams_in_workspace.is_empty() {
        let response = ApiResponse::<()>::forbidden("You don't have access to this workspace");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 确定用户在该workspace中的最高权限角色
    let user_role = user_teams_in_workspace
        .iter()
        .map(|(_, role)| role.as_str())
        .max_by(|a, b| {
            // 定义角色优先级：admin > manager > member
            let priority = |role: &str| match role {
                "admin" => 3,
                "manager" => 2,
                "member" => 1,
                _ => 0,
            };
            priority(a).cmp(&priority(b))
        })
        .unwrap_or("member")
        .to_string();

    // 保存之前的workspace_id
    let previous_workspace_id = current_user.current_workspace_id;

    // 更新用户的当前workspace
    let updated_rows = match diesel::update(schema::users::table)
        .filter(schema::users::id.eq(claims.sub))
        .set(schema::users::current_workspace_id.eq(Some(payload.workspace_id)))
        .execute(&mut conn)
    {
        Ok(rows) => rows,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update user workspace");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if updated_rows == 0 {
        let response = ApiResponse::<()>::internal_error("Failed to update user workspace");
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
    }

    // 构建响应数据
    let workspace_info = WorkspaceInfo {
        id: target_workspace.id,
        name: target_workspace.name.clone(),
        url_key: target_workspace.url_key.clone(),
    };

    let available_teams: Vec<TeamInfo> = user_teams_in_workspace
        .clone()
        .into_iter()
        .map(|(team, role)| TeamInfo {
            id: team.id,
            name: team.name,
            team_key: team.team_key,
            role,
        })
        .collect();

    let switch_result = WorkspaceSwitchResult {
        user_id: claims.sub,
        previous_workspace_id,
        current_workspace: workspace_info,
        user_role_in_workspace: user_role.clone(),
        available_teams,
    };

    let response = ApiResponse::success(switch_result, "Workspace switched successfully");
    (StatusCode::OK, Json(response)).into_response()
}

// OAuth相关路由
pub async fn oauth_authorize(Path(_provider): Path<String>) -> impl IntoResponse {
    // 这里应该重定向到OAuth提供商的授权页面
    // 暂时返回错误，后续实现
    let response = ApiResponse::<()>::not_implemented("OAuth authorization not implemented");
    (StatusCode::NOT_IMPLEMENTED, Json(response)).into_response()
}

pub async fn oauth_callback(
    Path(_provider): Path<String>,
    State(_pool): State<Arc<DbPool>>,
) -> impl IntoResponse {
    // 处理OAuth回调
    // 暂时返回错误，后续实现
    let response = ApiResponse::<()>::not_implemented("OAuth callback not implemented");
    (StatusCode::NOT_IMPLEMENTED, Json(response)).into_response()
}
