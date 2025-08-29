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

#[derive(Serialize)]
pub struct WorkspaceInvitationInfo {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub email: String,
    pub role: WorkspaceMemberRole,
    pub status: InvitationStatus,
    pub invited_by: UserBasicInfo,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct WorkspaceMembersAndInvitations {
    pub members: Vec<WorkspaceMemberInfo>,
    pub invitations: Vec<WorkspaceInvitationInfo>,
}

#[derive(Deserialize)]
pub struct InviteMemberRequest {
    pub email: String,
    pub role: WorkspaceMemberRole,
}

// 邀请成员加入工作区
pub async fn invite_member_to_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<InviteMemberRequest>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();
    let inviter_user_id = auth_info.user.id;

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

    // 检查是否已存在针对该邮箱的未处理邀请
    use crate::schema::invitations::dsl as inv_dsl;
    let existing_invitation = inv_dsl::invitations
        .filter(inv_dsl::workspace_id.eq(current_workspace_id))
        .filter(inv_dsl::email.eq(&payload.email))
        .filter(inv_dsl::status.eq(InvitationStatus::Pending))
        .filter(inv_dsl::expires_at.gt(chrono::Utc::now()))
        .select(Invitation::as_select())
        .first::<Invitation>(&mut conn);

    if existing_invitation.is_ok() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("email".to_string()),
            code: "PENDING_INVITATION".to_string(),
            message: "User already has a pending invitation to this workspace".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 创建邀请记录
    let new_invitation = NewInvitation {
        workspace_id: current_workspace_id,
        email: payload.email.clone(),
        role: payload.role.clone(),
        invited_by: inviter_user_id,
    };

    match diesel::insert_into(inv_dsl::invitations)
        .values(&new_invitation)
        .get_result::<Invitation>(&mut conn)
    {
        Ok(invitation) => {
            // TODO: 发送邀请邮件逻辑可以在这里添加

            let response = ApiResponse::success(
                Some(invitation),
                "Member invited successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to invite member to workspace");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

// 接受邀请
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

// 获取工作区成员列表（旧接口，只返回成员）
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

// 获取工作区成员和邀请列表（新接口）
pub async fn get_workspace_members_and_invitations(
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

    // 获取工作区成员
    use crate::schema::workspace_members::dsl as wm_dsl;
    use crate::schema::users::dsl as user_dsl;
    
    let members = match wm_dsl::workspace_members
        .filter(wm_dsl::workspace_id.eq(current_workspace_id))
        .inner_join(user_dsl::users.on(user_dsl::id.eq(wm_dsl::user_id)))
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

    // 获取工作区邀请
    use crate::schema::invitations::dsl as inv_dsl;
    let invitations = match inv_dsl::invitations
        .filter(inv_dsl::workspace_id.eq(current_workspace_id))
        .inner_join(user_dsl::users.on(user_dsl::id.eq(inv_dsl::invited_by)))
        .select((Invitation::as_select(), User::as_select()))
        .load::<(Invitation, User)>(&mut conn)
    {
        Ok(results) => results
            .into_iter()
            .map(|(invitation, user)| WorkspaceInvitationInfo {
                id: invitation.id,
                workspace_id: invitation.workspace_id,
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
            })
            .collect::<Vec<WorkspaceInvitationInfo>>(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve workspace invitations");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let result = WorkspaceMembersAndInvitations {
        members,
        invitations,
    };

    let response = ApiResponse::success(
        Some(result),
        "Workspace members and invitations retrieved successfully",
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

    // 获取工作区成员
    use crate::schema::workspace_members::dsl as wm_dsl;
    use crate::schema::users::dsl as user_dsl;
    
    let members = match wm_dsl::workspace_members
        .filter(wm_dsl::workspace_id.eq(current_workspace_id))
        .inner_join(user_dsl::users.on(user_dsl::id.eq(wm_dsl::user_id)))
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

    // 获取工作区邀请
    use crate::schema::invitations::dsl as inv_dsl;
    let invitations = match inv_dsl::invitations
        .filter(inv_dsl::workspace_id.eq(current_workspace_id))
        .inner_join(user_dsl::users.on(user_dsl::id.eq(inv_dsl::invited_by)))
        .select((Invitation::as_select(), User::as_select()))
        .load::<(Invitation, User)>(&mut conn)
    {
        Ok(results) => results
            .into_iter()
            .map(|(invitation, user)| WorkspaceInvitationInfo {
                id: invitation.id,
                workspace_id: invitation.workspace_id,
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
            })
            .collect::<Vec<WorkspaceInvitationInfo>>(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve workspace invitations");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let result = WorkspaceMembersAndInvitations {
        members,
        invitations,
    };

    let response = ApiResponse::success(
        Some(result),
        "Current workspace members and invitations retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}