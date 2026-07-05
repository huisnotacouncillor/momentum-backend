//! RBAC 中间件 - 工作区角色权限验证
//!
//! P0 修复：实现基于角色的访问控制（RBAC），防止越权操作
//!
//! ## 使用示例
//!
//! 在 handler 中手动调用权限检查：
//!
//! ```rust,ignore
//! async fn delete_workspace(
//!     State(state): State<Arc<AppState>>,
//!     user: AuthUserInfo,
//!     Path(workspace_id): Path<Uuid>,
//! ) -> Result<...> {
//!     check_workspace_role(&state, workspace_id, user.user.id, WorkspaceMemberRole::Owner, "Owner").await?;
//!     // 业务逻辑...
//! }
//! ```

use crate::middleware::auth::AuthUserInfo;
use crate::state::AppState;
use momentum_core::{
    db::{
        models::workspace_member::WorkspaceMemberRole,
        repositories::workspace_members::WorkspaceMembersRepo,
    },
    error::AppError,
};
use std::sync::Arc;
use uuid::Uuid;

/// 角色名称映射
pub fn role_name(role: &WorkspaceMemberRole) -> &'static str {
    match role {
        WorkspaceMemberRole::Owner => "Owner",
        WorkspaceMemberRole::Admin => "Admin",
        WorkspaceMemberRole::Member => "Member",
        WorkspaceMemberRole::Guest => "Guest",
    }
}

/// 检查用户在工作区的角色
///
/// 返回 Ok(()) 表示通过检查，返回 Err(AppError::Forbidden) 表示无权访问
pub async fn check_workspace_role(
    state: &Arc<AppState>,
    workspace_id: Uuid,
    user_id: Uuid,
    required: WorkspaceMemberRole,
    role_label: &str,
) -> Result<(), AppError> {
    let mut conn = state
        .db
        .get()
        .map_err(|_| AppError::ServiceUnavailable {
            message: "Database temporarily unavailable".to_string(),
        })?;

    let membership = WorkspaceMembersRepo::find(&mut conn, workspace_id, user_id)
        .map_err(AppError::Database)?;

    match membership {
        Some(m) if m.role.has_at_least(&required) => Ok(()),
        Some(_) => Err(AppError::forbidden(format!(
            "{} role required",
            role_label
        ))),
        None => Err(AppError::forbidden("Not a member of this workspace")),
    }
}

/// 便利函数：检查用户是否为 Owner
pub async fn require_owner(
    state: &Arc<AppState>,
    workspace_id: Uuid,
    user: &AuthUserInfo,
) -> Result<(), AppError> {
    check_workspace_role(
        state,
        workspace_id,
        user.user.id,
        WorkspaceMemberRole::Owner,
        role_name(&WorkspaceMemberRole::Owner),
    )
    .await
}

/// 便利函数：检查用户是否为 Admin 或更高
pub async fn require_admin(
    state: &Arc<AppState>,
    workspace_id: Uuid,
    user: &AuthUserInfo,
) -> Result<(), AppError> {
    check_workspace_role(
        state,
        workspace_id,
        user.user.id,
        WorkspaceMemberRole::Admin,
        role_name(&WorkspaceMemberRole::Admin),
    )
    .await
}

/// 便利函数：检查用户是否为 Member 或更高
pub async fn require_member(
    state: &Arc<AppState>,
    workspace_id: Uuid,
    user: &AuthUserInfo,
) -> Result<(), AppError> {
    check_workspace_role(
        state,
        workspace_id,
        user.user.id,
        WorkspaceMemberRole::Member,
        role_name(&WorkspaceMemberRole::Member),
    )
    .await
}

/// 检查用户是否属于指定工作区（任何角色）
pub async fn check_workspace_membership(
    state: &Arc<AppState>,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    check_workspace_role(
        state,
        workspace_id,
        user_id,
        WorkspaceMemberRole::Guest,
        role_name(&WorkspaceMemberRole::Guest),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_hierarchy() {
        assert!(WorkspaceMemberRole::Owner.has_at_least(&WorkspaceMemberRole::Admin));
        assert!(WorkspaceMemberRole::Admin.has_at_least(&WorkspaceMemberRole::Member));
        assert!(WorkspaceMemberRole::Member.has_at_least(&WorkspaceMemberRole::Guest));
        assert!(!WorkspaceMemberRole::Member.has_at_least(&WorkspaceMemberRole::Owner));
        assert!(!WorkspaceMemberRole::Guest.has_at_least(&WorkspaceMemberRole::Member));
    }

    #[test]
    fn test_role_levels() {
        assert_eq!(WorkspaceMemberRole::Owner.level(), 4);
        assert_eq!(WorkspaceMemberRole::Admin.level(), 3);
        assert_eq!(WorkspaceMemberRole::Member.level(), 2);
        assert_eq!(WorkspaceMemberRole::Guest.level(), 1);
    }

    #[test]
    fn test_role_name() {
        assert_eq!(role_name(&WorkspaceMemberRole::Owner), "Owner");
        assert_eq!(role_name(&WorkspaceMemberRole::Admin), "Admin");
        assert_eq!(role_name(&WorkspaceMemberRole::Member), "Member");
        assert_eq!(role_name(&WorkspaceMemberRole::Guest), "Guest");
    }
}