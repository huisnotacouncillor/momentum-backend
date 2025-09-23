use crate::db::models::{ApiResponse, ErrorDetail, TeamInfo, TeamMemberInfo};
use crate::db::{DbPool, models::*};
use crate::middleware::auth::AuthUserInfo;
use crate::schema;
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
    let user_id = auth_info.user.id;
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证团队名称不为空
    if payload.name.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("name".to_string()),
            code: "REQUIRED".to_string(),
            message: "Team name is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 验证 team_key 不为空且格式正确
    if payload.team_key.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("team_key".to_string()),
            code: "REQUIRED".to_string(),
            message: "Team key is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 检查 team_key 格式
    if !payload
        .team_key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("team_key".to_string()),
            code: "INVALID_FORMAT".to_string(),
            message: "Team key can only contain letters, numbers, hyphens, and underscores"
                .to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 开始事务
    let result = conn.transaction::<Team, diesel::result::Error, _>(|conn| {
        // 创建团队
        let new_team = NewTeam {
            name: payload.name.clone(),
            team_key: payload.team_key.clone(),
            description: payload.description.clone(),
            icon_url: payload.icon_url.clone(),
            is_private: payload.is_private,
            workspace_id: current_workspace_id,
        };

        let team: Team = diesel::insert_into(schema::teams::table)
            .values(&new_team)
            .get_result::<Team>(conn)?;

        // 创建团队成员关系（创建者自动成为管理员）
        let new_team_member = NewTeamMember {
            user_id,
            team_id: team.id,
            role: "admin".to_string(),
        };

        diesel::insert_into(schema::team_members::table)
            .values(&new_team_member)
            .execute(conn)?;

        // TODO: 创建默认工作流和状态
        // create_default_workflow_for_team(conn, team.id)?;

        Ok(team)
    });

    match result {
        Ok(team) => {
            let response = ApiResponse::success(Some(team), "Team created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            // 检查是否是唯一性约束违反错误
            if e.to_string().contains("team_key") {
                let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                    field: Some("team_key".to_string()),
                    code: "DUPLICATE".to_string(),
                    message: "Team with this key already exists in this workspace".to_string(),
                }]);
                (StatusCode::BAD_REQUEST, Json(response)).into_response()
            } else {
                let response = ApiResponse::<()>::internal_error("Failed to create team");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
        }
    }
}

/// 获取团队列表
pub async fn get_teams(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    use crate::schema::teams::dsl::*;
    let teams_list = match teams
        .filter(workspace_id.eq(current_workspace_id))
        .select(Team::as_select())
        .order(created_at.desc())
        .load::<Team>(&mut conn)
    {
        Ok(list) => list,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve teams");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(Some(teams_list), "Teams retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

/// 获取指定团队
pub async fn get_team(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    use crate::schema::teams::dsl::*;
    let team = match teams
        .filter(id.eq(team_id))
        .filter(workspace_id.eq(current_workspace_id))
        .select(Team::as_select())
        .first::<Team>(&mut conn)
    {
        Ok(team) => team,
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(Some(team), "Team retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

/// 更新团队
pub async fn update_team(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<UpdateTeamRequest>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查团队是否存在且属于当前工作区
    use crate::schema::teams::dsl::*;
    let existing_team = match teams
        .filter(id.eq(team_id))
        .filter(workspace_id.eq(current_workspace_id))
        .select(Team::as_select())
        .first::<Team>(&mut conn)
    {
        Ok(team) => team,
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    // 如果没有要更新的字段，则返回原始团队信息
    if payload.name.is_none()
        && payload.team_key.is_none()
        && payload.description.is_none()
        && payload.icon_url.is_none()
        && payload.is_private.is_none()
    {
        let response = ApiResponse::success(Some(existing_team), "Team retrieved successfully");
        return (StatusCode::OK, Json(response)).into_response();
    }

    // 构建更新查询，使用现有值作为默认值
    let team_name = payload.name.as_ref().unwrap_or(&existing_team.name);
    let team_key_val = payload.team_key.as_ref().unwrap_or(&existing_team.team_key);
    let description_val = payload
        .description
        .as_ref()
        .or(existing_team.description.as_ref());
    let icon_url_val = payload
        .icon_url
        .as_ref()
        .or(existing_team.icon_url.as_ref());
    let is_private_val = payload.is_private.unwrap_or(existing_team.is_private);

    match diesel::update(schema::teams::table.filter(schema::teams::id.eq(team_id)))
        .set((
            schema::teams::name.eq(team_name),
            schema::teams::team_key.eq(team_key_val),
            schema::teams::description.eq(description_val),
            schema::teams::icon_url.eq(icon_url_val),
            schema::teams::is_private.eq(is_private_val),
        ))
        .get_result::<Team>(&mut conn)
    {
        Ok(updated_team) => {
            let response = ApiResponse::success(Some(updated_team), "Team updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            // 检查是否是唯一性约束违反错误
            if e.to_string().contains("team_key") {
                let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                    field: Some("team_key".to_string()),
                    code: "DUPLICATE".to_string(),
                    message: "Team with this key already exists in this workspace".to_string(),
                }]);
                (StatusCode::BAD_REQUEST, Json(response)).into_response()
            } else {
                let response = ApiResponse::<()>::internal_error("Failed to update team");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
        }
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

    // 检查团队是否存在且属于当前工作区
    use crate::schema::teams::dsl as team_dsl;
    let current_workspace_id = auth_info.current_workspace_id.unwrap();
    match crate::schema::teams::dsl::teams
        .filter(team_dsl::id.eq(team_id))
        .filter(team_dsl::workspace_id.eq(current_workspace_id))
        .select(Team::as_select())
        .first::<Team>(&mut conn)
    {
        Ok(_) => (),
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    }

    // 删除团队
    match diesel::delete(
        crate::schema::teams::dsl::teams.filter(crate::schema::teams::dsl::id.eq(team_id)),
    )
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

    // 检查团队是否存在且属于当前工作区
    match schema::teams::table
        .filter(schema::teams::id.eq(team_id))
        .filter(schema::teams::workspace_id.eq(current_workspace_id))
        .select(Team::as_select())
        .first::<Team>(&mut conn)
    {
        Ok(_) => (),
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    }

    // 验证用户是否是工作区成员
    use crate::schema::workspace_members::dsl::*;
    match workspace_members
        .filter(user_id.eq(payload.user_id))
        .filter(crate::schema::workspace_members::dsl::workspace_id.eq(current_workspace_id))
        .select(WorkspaceMember::as_select())
        .first::<WorkspaceMember>(&mut conn)
    {
        Ok(_) => (),
        Err(_) => {
            let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                field: Some("user_id".to_string()),
                code: "INVALID".to_string(),
                message: "User is not a member of this workspace".to_string(),
            }]);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    }

    // 使用枚举类型直接获取角色值
    let other_role = payload.role;
    // 验证角色是否有效
    let role_str = match other_role {
        TeamRole::Admin => "admin".to_string(),
        TeamRole::Member => "member".to_string(),
    };

    // 添加团队成员
    let new_team_member = NewTeamMember {
        user_id: payload.user_id,
        team_id: team_id,
        role: role_str,
    };

    match diesel::insert_into(schema::team_members::table)
        .values(&new_team_member)
        .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Team member added successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            // 检查是否是唯一性约束违反错误
            if e.to_string().contains("team_members_user_id_team_id_key") {
                let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                    field: Some("user_id".to_string()),
                    code: "DUPLICATE".to_string(),
                    message: "User is already a member of this team".to_string(),
                }]);
                (StatusCode::BAD_REQUEST, Json(response)).into_response()
            } else {
                let response = ApiResponse::<()>::internal_error("Failed to add team member");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
        }
    }
}

/// 获取团队成员列表
pub async fn get_team_members_list(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查团队是否存在且属于当前工作区
    use crate::schema::teams::dsl::*;
    match teams
        .filter(id.eq(team_id))
        .filter(workspace_id.eq(current_workspace_id))
        .select(Team::as_select())
        .first::<Team>(&mut conn)
    {
        Ok(_) => (),
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    }

    // 获取团队成员列表
    let members = match schema::team_members::table
        .filter(schema::team_members::team_id.eq(team_id))
        .inner_join(schema::users::table.on(schema::users::id.eq(schema::team_members::user_id)))
        .filter(schema::team_members::team_id.eq(team_id))
        .select((TeamMember::as_select(), User::as_select()))
        .load::<(TeamMember, User)>(&mut conn)
    {
        Ok(results) => results
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
            .collect::<Vec<TeamMemberInfo>>(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve team members");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

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

    // 检查团队是否存在且属于当前工作区
    use crate::schema::teams::dsl::*;
    match teams
        .filter(id.eq(team_id))
        .filter(workspace_id.eq(current_workspace_id))
        .select(Team::as_select())
        .first::<Team>(&mut conn)
    {
        Ok(_) => (),
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    }

    // 验证团队成员是否存在
    use crate::schema::team_members::dsl as team_member_dsl;
    match crate::schema::team_members::dsl::team_members
        .filter(team_member_dsl::team_id.eq(team_id))
        .filter(team_member_dsl::user_id.eq(member_user_id))
        .select(TeamMember::as_select())
        .first::<TeamMember>(&mut conn)
    {
        Ok(_) => (),
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team member not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    }

    // 使用枚举类型直接获取角色值
    let role = payload.role;

    // 更新团队成员角色
    match diesel::update(
        crate::schema::team_members::dsl::team_members
            .filter(crate::schema::team_members::dsl::team_id.eq(team_id))
            .filter(crate::schema::team_members::dsl::user_id.eq(member_user_id)),
    )
    .set(schema::team_members::role.eq(match role {
        TeamRole::Admin => "admin".to_string(),
        TeamRole::Member => "member".to_string(),
    }))
    .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Team member updated successfully");
            (StatusCode::OK, Json(response)).into_response()
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

    // 检查团队是否存在且属于当前工作区
    use crate::schema::teams::dsl::*;
    match teams
        .filter(id.eq(team_id))
        .filter(workspace_id.eq(current_workspace_id))
        .select(Team::as_select())
        .first::<Team>(&mut conn)
    {
        Ok(_) => (),
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    }

    // 验证团队成员是否存在
    use crate::schema::team_members::dsl as team_member_dsl;
    match crate::schema::team_members::dsl::team_members
        .filter(team_member_dsl::team_id.eq(team_id))
        .filter(team_member_dsl::user_id.eq(member_user_id))
        .select(TeamMember::as_select())
        .first::<TeamMember>(&mut conn)
    {
        Ok(_) => (),
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Team member not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    }

    // 移除团队成员
    match diesel::delete(
        crate::schema::team_members::dsl::team_members
            .filter(crate::schema::team_members::dsl::team_id.eq(team_id))
            .filter(crate::schema::team_members::dsl::user_id.eq(member_user_id)),
    )
    .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Team member removed successfully");
            (StatusCode::OK, Json(response)).into_response()
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
