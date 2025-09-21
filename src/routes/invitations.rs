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

// 邀请成员请求体
#[derive(Deserialize)]
pub struct InviteMemberRequest {
    pub emails: Vec<String>,
    #[serde(default)]
    pub role: Option<WorkspaceMemberRole>,
}

/// 邀请用户加入当前工作区
///
/// 权限要求: 当前用户必须是工作区的Owner或Admin
pub async fn invite_member(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<InviteMemberRequest>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();
    let inviter_user_id = auth_info.user.id;

    // 验证请求数据
    if payload.emails.is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("emails".to_string()),
            code: "EMPTY_EMAILS".to_string(),
            message: "Emails list cannot be empty".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查邀请人是否是工作区成员，并且有足够权限（至少是Admin）
    use crate::schema::workspace_members::dsl as wm_dsl;
    let inviter_member = match wm_dsl::workspace_members
        .filter(wm_dsl::workspace_id.eq(current_workspace_id))
        .filter(wm_dsl::user_id.eq(inviter_user_id))
        .select(WorkspaceMember::as_select())
        .first::<WorkspaceMember>(&mut conn)
    {
        Ok(member) => member,
        Err(_) => {
            let response = ApiResponse::<()>::forbidden("Only workspace members can invite others");
            return (StatusCode::FORBIDDEN, Json(response)).into_response();
        }
    };

    // 检查邀请人权限（只有Owner和Admin可以邀请成员）
    match inviter_member.role {
        WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin => (),
        _ => {
            let response = ApiResponse::<()>::forbidden("Insufficient permissions to invite members");
            return (StatusCode::FORBIDDEN, Json(response)).into_response();
        }
    }

    // 确定角色，默认为Member
    let role = payload.role.unwrap_or(WorkspaceMemberRole::Member);

    let mut created_invitations = Vec::new();
    let mut errors = Vec::new();

    // 为每个邮箱创建邀请
    for email in &payload.emails {
        // 验证邮箱格式（简单验证）
        if !is_valid_email(email) {
            errors.push(ErrorDetail {
                field: Some("emails".to_string()),
                code: "INVALID_EMAIL".to_string(),
                message: format!("Invalid email format: {}", email),
            });
            continue;
        }

        // 检查是否已存在针对该邮箱的未处理邀请
        use crate::schema::invitations::dsl as inv_dsl;
        let existing_invitation = inv_dsl::invitations
            .filter(inv_dsl::workspace_id.eq(current_workspace_id))
            .filter(inv_dsl::email.eq(email))
            .filter(inv_dsl::status.eq(InvitationStatus::Pending))
            .filter(inv_dsl::expires_at.gt(chrono::Utc::now()))
            .select(Invitation::as_select())
            .first::<Invitation>(&mut conn);

        if existing_invitation.is_ok() {
            errors.push(ErrorDetail {
                field: Some("emails".to_string()),
                code: "PENDING_INVITATION".to_string(),
                message: format!("User {} already has a pending invitation to this workspace", email),
            });
            continue;
        }

        // 创建邀请记录
        let new_invitation = NewInvitation {
            workspace_id: current_workspace_id,
            email: email.clone(),
            role: role.clone(),
            invited_by: inviter_user_id,
        };

        match diesel::insert_into(inv_dsl::invitations)
            .values(&new_invitation)
            .get_result::<Invitation>(&mut conn)
        {
            Ok(invitation) => {
                created_invitations.push(invitation);
            }
            Err(e) => {
                errors.push(ErrorDetail {
                    field: Some("emails".to_string()),
                    code: "INVITATION_FAILED".to_string(),
                    message: format!("Failed to invite {}: {}", email, e),
                });
            }
        }
    }

    // 构建响应
    if created_invitations.is_empty() && !errors.is_empty() {
        // 所有邀请都失败了
        let response = ApiResponse::<()>::validation_error(errors);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    } else if !errors.is_empty() {
        // 部分邀请失败
        let response = ApiResponse::success(
            Some(created_invitations),
            "Some invitations were created successfully",
        );

        // 将错误信息添加到响应meta中
        let response_with_errors = ApiResponse {
            errors: Some(errors),
            ..response
        };
        (StatusCode::CREATED, Json(response_with_errors)).into_response()
    } else {
        // 所有邀请都成功
        let response = ApiResponse::created(
            Some(created_invitations),
            "Members invited successfully",
        );
        (StatusCode::CREATED, Json(response)).into_response()
    }
}

// 简单的邮箱格式验证
fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() > 5
}

/// 获取当前用户的邀请列表
pub async fn get_user_invitations(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let user_email = auth_info.user.email;

    // 创建资源 URL 处理工具
    let asset_helper = &state.asset_helper;

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 获取用户收到的邀请
    use crate::schema::invitations::dsl as inv_dsl;
    use crate::schema::users::dsl as user_dsl;
    use crate::schema::workspaces::dsl as workspace_dsl;

    let invitations = match inv_dsl::invitations
        .filter(inv_dsl::email.eq(user_email))
        .inner_join(user_dsl::users.on(user_dsl::id.eq(inv_dsl::invited_by)))
        .inner_join(workspace_dsl::workspaces.on(workspace_dsl::id.eq(inv_dsl::workspace_id)))
        .select((Invitation::as_select(), User::as_select(), Workspace::as_select()))
        .load::<(Invitation, User, Workspace)>(&mut conn)
    {
        Ok(results) => results
            .into_iter()
            .map(|(invitation, user, workspace)| InvitationDetail {
                id: invitation.id,
                workspace_id: invitation.workspace_id,
                workspace_name: workspace.name.clone(),
                workspace: workspace,
                email: invitation.email,
                role: invitation.role,
                status: invitation.status,
                invited_by: {
                    let processed_avatar_url = user.get_processed_avatar_url(&asset_helper);
                    UserBasicInfo {
                        id: user.id,
                        name: user.name,
                        username: user.username,
                        email: user.email,
                        avatar_url: processed_avatar_url,
                    }
                },
                created_at: invitation.created_at,
                updated_at: invitation.updated_at,
                expires_at: invitation.expires_at,
            })
            .collect::<Vec<InvitationDetail>>(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve invitations");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(invitations),
        "User invitations retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

/// 获取特定邀请的详细信息
pub async fn get_invitation_by_id(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(invitation_id): Path<Uuid>,
) -> impl IntoResponse {
    let user_email = auth_info.user.email;

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 获取特定邀请
    use crate::schema::invitations::dsl as inv_dsl;
    use crate::schema::users::dsl as user_dsl;
    use crate::schema::workspaces::dsl as workspace_dsl;

    let invitation_result = match inv_dsl::invitations
        .filter(inv_dsl::id.eq(invitation_id))
        .filter(inv_dsl::email.eq(user_email.clone())) // 确保用户只能查看自己的邀请
        .inner_join(user_dsl::users.on(user_dsl::id.eq(inv_dsl::invited_by)))
        .inner_join(workspace_dsl::workspaces.on(workspace_dsl::id.eq(inv_dsl::workspace_id)))
        .select((Invitation::as_select(), User::as_select(), Workspace::as_select()))
        .first::<(Invitation, User, Workspace)>(&mut conn)
    {
        Ok(result) => result,
        Err(diesel::NotFound) => {
            let response = ApiResponse::<()>::not_found("Invitation not found or not accessible");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve invitation");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let (invitation, user, workspace) = invitation_result;
    // 确保用户只能查看自己的邀请
    if invitation.email != user_email {
        let response = ApiResponse::<()>::forbidden("You are not authorized to view this invitation");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }
    let invitation_detail = InvitationDetail {
        id: invitation.id,
        workspace_id: invitation.workspace_id,
        workspace_name: workspace.name.clone(),
        workspace: workspace,
        email: invitation.email,
        role: invitation.role,
        status: invitation.status,
        invited_by: UserBasicInfo {
            id: user.id,
            name: user.name,
            username: user.username,
            email: user.email,
            avatar_url: user.avatar_url,
        },
        created_at: invitation.created_at,
        updated_at: invitation.updated_at,
        expires_at: invitation.expires_at,
    };

    let response = ApiResponse::success(
        Some(invitation_detail), // 包装成数组以符合API规范
        "Invitation retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

/// 接受邀请
pub async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(invitation_id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = auth_info.user.id;
    let user_email = auth_info.user.email.clone();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 查找邀请
    use crate::schema::invitations::dsl as inv_dsl;
    let invitation = match inv_dsl::invitations
        .filter(inv_dsl::id.eq(invitation_id))
        .select(Invitation::as_select())
        .first::<Invitation>(&mut conn)
    {
        Ok(invitation) => invitation,
        Err(diesel::NotFound) => {
            let response = ApiResponse::<()>::not_found("Invitation not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve invitation");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证邀请是否有效
    if invitation.status != InvitationStatus::Pending {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: None,
            code: "INVITATION_NOT_PENDING".to_string(),
            message: "Invitation is no longer pending".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    if invitation.expires_at < chrono::Utc::now() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: None,
            code: "INVITATION_EXPIRED".to_string(),
            message: "Invitation has expired".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    if invitation.email != user_email {
        let response = ApiResponse::<()>::forbidden("You are not authorized to accept this invitation");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 开始事务
    let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // 更新邀请状态为已接受
        let updated_invitation = diesel::update(inv_dsl::invitations.filter(inv_dsl::id.eq(invitation_id)))
            .set((
                inv_dsl::status.eq(InvitationStatus::Accepted),
                inv_dsl::updated_at.eq(chrono::Utc::now())
            ))
            .get_result::<Invitation>(conn)?;

        // 创建工作区成员关系
        let new_workspace_member = NewWorkspaceMember {
            user_id,
            workspace_id: invitation.workspace_id,
            role: invitation.role.clone(),
        };

        diesel::insert_into(crate::schema::workspace_members::table)
            .values(&new_workspace_member)
            .execute(conn)?;

        // 将用户添加到工作区的默认团队中
        // 首先找到工作区的默认团队（通常第一个创建的团队）
        let default_team = crate::schema::teams::table
            .filter(crate::schema::teams::workspace_id.eq(invitation.workspace_id))
            .order(crate::schema::teams::created_at.asc())
            .select(Team::as_select())
            .first::<Team>(conn)?;

        // 将用户添加为该团队的成员
        let new_team_member = NewTeamMember {
            user_id,
            team_id: default_team.id,
            role: "member".to_string(), // 默认为成员角色
        };

        diesel::insert_into(crate::schema::team_members::table)
            .values(&new_team_member)
            .execute(conn)?;

        // 如果用户没有当前工作区，设置这个工作区为当前工作区
        use crate::schema::users::dsl as user_dsl;
        let user = user_dsl::users
            .filter(user_dsl::id.eq(user_id))
            .select(User::as_select())
            .first::<User>(conn)?;

        if user.current_workspace_id.is_none() {
            diesel::update(user_dsl::users.filter(user_dsl::id.eq(user_id)))
                .set(user_dsl::current_workspace_id.eq(invitation.workspace_id))
                .execute(conn)?;
        }

        Ok(updated_invitation)
    });

    match result {
        Ok(updated_invitation) => {
            let response = ApiResponse::success(
                Some(updated_invitation),
                "Invitation accepted successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to accept invitation");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 拒绝邀请
pub async fn decline_invitation(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(invitation_id): Path<Uuid>,
) -> impl IntoResponse {
    let user_email = auth_info.user.email;

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 查找邀请
    use crate::schema::invitations::dsl as inv_dsl;
    let invitation = match inv_dsl::invitations
        .filter(inv_dsl::id.eq(invitation_id))
        .select(Invitation::as_select())
        .first::<Invitation>(&mut conn)
    {
        Ok(invitation) => invitation,
        Err(diesel::NotFound) => {
            let response = ApiResponse::<()>::not_found("Invitation not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve invitation");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证邀请是否有效
    if invitation.status != InvitationStatus::Pending {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: None,
            code: "INVITATION_NOT_PENDING".to_string(),
            message: "Invitation is no longer pending".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    if invitation.expires_at < chrono::Utc::now() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: None,
            code: "INVITATION_EXPIRED".to_string(),
            message: "Invitation has expired".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    if invitation.email != user_email {
        let response = ApiResponse::<()>::forbidden("You are not authorized to decline this invitation");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }

    // 更新邀请状态为已拒绝
    match diesel::update(inv_dsl::invitations.filter(inv_dsl::id.eq(invitation_id)))
        .set((
            inv_dsl::status.eq(InvitationStatus::Declined),
            inv_dsl::updated_at.eq(chrono::Utc::now())
        ))
        .get_result::<Invitation>(&mut conn)
    {
        Ok(updated_invitation) => {
            let response = ApiResponse::success(
                Some(updated_invitation),
                "Invitation declined successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to decline invitation");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 撤销邀请（由邀请人操作）
///
/// 权限要求: 当前用户必须是邀请人或工作区的Owner/Admin
pub async fn revoke_invitation(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(invitation_id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = auth_info.user.id;

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 查找邀请
    use crate::schema::invitations::dsl as inv_dsl;
    let invitation = match inv_dsl::invitations
        .filter(inv_dsl::id.eq(invitation_id))
        .select(Invitation::as_select())
        .first::<Invitation>(&mut conn)
    {
        Ok(invitation) => invitation,
        Err(diesel::NotFound) => {
            let response = ApiResponse::<()>::not_found("Invitation not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve invitation");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查权限：必须是邀请人或工作区Owner/Admin
    if invitation.invited_by != user_id {
        // 不是邀请人，检查是否是工作区Owner/Admin
        use crate::schema::workspace_members::dsl as wm_dsl;
        let member = match wm_dsl::workspace_members
            .filter(wm_dsl::workspace_id.eq(invitation.workspace_id))
            .filter(wm_dsl::user_id.eq(user_id))
            .select(WorkspaceMember::as_select())
            .first::<WorkspaceMember>(&mut conn)
        {
            Ok(member) => member,
            Err(_) => {
                let response = ApiResponse::<()>::forbidden("You are not authorized to revoke this invitation");
                return (StatusCode::FORBIDDEN, Json(response)).into_response();
            }
        };

        match member.role {
            WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin => (),
            _ => {
                let response = ApiResponse::<()>::forbidden("You are not authorized to revoke this invitation");
                return (StatusCode::FORBIDDEN, Json(response)).into_response();
            }
        }
    }

    // 验证邀请是否可以撤销
    if invitation.status != InvitationStatus::Pending {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: None,
            code: "INVITATION_NOT_PENDING".to_string(),
            message: "Invitation is no longer pending".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 更新邀请状态为已撤销
    match diesel::update(inv_dsl::invitations.filter(inv_dsl::id.eq(invitation_id)))
        .set((
            inv_dsl::status.eq(InvitationStatus::Cancelled),
            inv_dsl::updated_at.eq(chrono::Utc::now())
        ))
        .get_result::<Invitation>(&mut conn)
    {
        Ok(updated_invitation) => {
            let response = ApiResponse::success(
                Some(updated_invitation),
                "Invitation revoked successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to revoke invitation");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

// 邀请详细信息结构
#[derive(Serialize)]
pub struct InvitationDetail {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub workspace_name: String,
    pub workspace: Workspace,
    pub email: String,
    pub role: WorkspaceMemberRole,
    pub status: InvitationStatus,
    pub invited_by: UserBasicInfo,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}