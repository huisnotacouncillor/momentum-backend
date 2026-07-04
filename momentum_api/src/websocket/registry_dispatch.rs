//! 双轨 dispatch helper（spec §3 + 旧 handler 回退）
//!
//! 设计目的：让 `WebSocketCommandHandler::handle_command` 在旧 match
//! 之前**先**问 registry：registry 里有 command_type 就走 trait；
//! 没有则返回 `None`，调用方继续走旧路径（不破坏现有行为）。
//!
//! 关键约束：现有**任何**代码路径不允许因这个 helper 引入回归。
//! 所以：
//! - 不修改 `commands/handler.rs` 任何字段；
//! - 旧路径完整保留；
//! - 仅在订阅类（subscribe/unsubscribe）等需要"连接级状态"的命令上
//!   显式跳过（这些尚未接到 SessionManager，由旧 handler 兜底）。

use std::sync::Arc;

use serde_json::Value;

use momentum_core::services::context::RequestContext;

use crate::websocket::auth::AuthenticatedUser;
use crate::websocket::commands::types::{
    WebSocketCommand, WebSocketCommandError, WebSocketCommandResponse,
};
use crate::websocket::registry::{HandlerError, HandlerRegistry};

/// Step 8 skip list：这些命令需要连接级状态（SessionManager）才能正确处理；
/// 在 SessionManager 接入之前不通过 registry 路由。
///
/// 如果将来把所有命令都迁到 registry 且 SessionManager 就绪，
/// 可以把这个 list 变成空集。
const REGISTRY_SKIPLIST: &[&str] = &["subscribe", "unsubscribe"];

/// 优先尝试 registry；如果 registry 没有该命令、或在 skip list 中，返回 None。
///
/// 调用方应该这样使用：
/// ```ignore
/// pub async fn handle_command(&self, command, user) -> WebSocketCommandResponse {
///     // 注册到 AppState 时通过 `with_registry()` 安装
///     if let Some(reg) = self.registry.as_ref() {
///         if let Some(resp) = registry_dispatch::try_dispatch(reg, &command, user).await {
///             return resp;
///         }
///     }
///     self.handle_command_legacy(command, user).await
/// }
/// ```
pub async fn try_dispatch(
    reg: &HandlerRegistry,
    command: &WebSocketCommand,
    user: &AuthenticatedUser,
) -> Option<WebSocketCommandResponse> {
    let command_type = command_type_of(command);
    if REGISTRY_SKIPLIST.contains(&command_type) {
        return None;
    }
    if !reg.registered_types().iter().any(|t| *t == command_type) {
        return None;
    }

    let payload = to_payload(command);
    let ctx = RequestContext {
        user_id: user.user_id,
        workspace_id: user.current_workspace_id.unwrap_or_else(uuid_nil),
        idempotency_key: None,
    };
    let request_id = request_id_of(command);

    match reg.dispatch(command_type, ctx, payload) {
        Ok(value) => Some(WebSocketCommandResponse::success(
            command_type,
            "registry",
            request_id,
            value,
        )),
        Err(HandlerError::NotFound { command_type }) => {
            // 在我们 already 检查过 registered_types 的前提下不应到这里；
            // 防御性兜底：仍由旧路径处理
            tracing::warn!(
                "registry race: command '{}' disappeared from registry",
                command_type
            );
            None
        }
        Err(HandlerError::FeatureDisabled { feature }) => {
            Some(WebSocketCommandResponse::error(
                command_type,
                "registry",
                request_id,
                WebSocketCommandError::business_error(
                    "FEATURE_FLAG_DISABLED",
                    &format!("feature disabled: {feature}"),
                ),
            ))
        }
        Err(HandlerError::Internal { detail }) => Some(WebSocketCommandResponse::error(
            command_type,
            "registry",
            request_id,
            WebSocketCommandError::system_error(&detail),
        )),
    }
}

/// 从 enum 推 command_type 字符串（与 commands/handler.rs::handle_command 内部一致）
fn command_type_of(cmd: &WebSocketCommand) -> &'static str {
    match cmd {
        WebSocketCommand::CreateLabel { .. } => "create_label",
        WebSocketCommand::UpdateLabel { .. } => "update_label",
        WebSocketCommand::DeleteLabel { .. } => "delete_label",
        WebSocketCommand::QueryLabels { .. } => "query_labels",
        WebSocketCommand::BatchCreateLabels { .. } => "batch_create_labels",
        WebSocketCommand::BatchUpdateLabels { .. } => "batch_update_labels",
        WebSocketCommand::BatchDeleteLabels { .. } => "batch_delete_labels",
        WebSocketCommand::Subscribe { .. } => "subscribe",
        WebSocketCommand::Unsubscribe { .. } => "unsubscribe",
        WebSocketCommand::GetConnectionInfo { .. } => "get_connection_info",
        WebSocketCommand::Ping { .. } => "ping",
        WebSocketCommand::CreateTeam { .. } => "create_team",
        WebSocketCommand::UpdateTeam { .. } => "update_team",
        WebSocketCommand::DeleteTeam { .. } => "delete_team",
        WebSocketCommand::QueryTeams { .. } => "query_teams",
        WebSocketCommand::AddTeamMember { .. } => "add_team_member",
        WebSocketCommand::UpdateTeamMember { .. } => "update_team_member",
        WebSocketCommand::RemoveTeamMember { .. } => "remove_team_member",
        WebSocketCommand::ListTeamMembers { .. } => "list_team_members",
        WebSocketCommand::InviteWorkspaceMember { .. } => "invite_workspace_member",
        WebSocketCommand::AcceptInvitation { .. } => "accept_invitation",
        WebSocketCommand::QueryWorkspaceMembers { .. } => "query_workspace_members",
        WebSocketCommand::CreateProjectStatus { .. } => "create_project_status",
        WebSocketCommand::UpdateProjectStatus { .. } => "update_project_status",
        WebSocketCommand::DeleteProjectStatus { .. } => "delete_project_status",
        WebSocketCommand::QueryProjectStatuses { .. } => "query_project_statuses",
        WebSocketCommand::GetProjectStatusById { .. } => "get_project_status_by_id",
        WebSocketCommand::CreateWorkspace { .. } => "create_workspace",
        WebSocketCommand::UpdateWorkspace { .. } => "update_workspace",
        WebSocketCommand::DeleteWorkspace { .. } => "delete_workspace",
        WebSocketCommand::GetCurrentWorkspace { .. } => "get_current_workspace",
        WebSocketCommand::UpdateProfile { .. } => "update_profile",
        WebSocketCommand::CreateProject { .. } => "create_project",
        WebSocketCommand::UpdateProject { .. } => "update_project",
        WebSocketCommand::DeleteProject { .. } => "delete_project",
        WebSocketCommand::QueryProjects { .. } => "query_projects",
        WebSocketCommand::CreateIssue { .. } => "create_issue",
        WebSocketCommand::UpdateIssue { .. } => "update_issue",
        WebSocketCommand::DeleteIssue { .. } => "delete_issue",
        WebSocketCommand::QueryIssues { .. } => "query_issues",
        WebSocketCommand::GetIssue { .. } => "get_issue",
    }
}

fn request_id_of(cmd: &WebSocketCommand) -> Option<String> {
    let id = match cmd {
        WebSocketCommand::CreateLabel { request_id, .. }
        | WebSocketCommand::UpdateLabel { request_id, .. }
        | WebSocketCommand::DeleteLabel { request_id, .. }
        | WebSocketCommand::QueryLabels { request_id, .. }
        | WebSocketCommand::BatchCreateLabels { request_id, .. }
        | WebSocketCommand::BatchUpdateLabels { request_id, .. }
        | WebSocketCommand::BatchDeleteLabels { request_id, .. }
        | WebSocketCommand::Subscribe { request_id, .. }
        | WebSocketCommand::Unsubscribe { request_id, .. }
        | WebSocketCommand::GetConnectionInfo { request_id, .. }
        | WebSocketCommand::Ping { request_id, .. }
        | WebSocketCommand::CreateTeam { request_id, .. }
        | WebSocketCommand::UpdateTeam { request_id, .. }
        | WebSocketCommand::DeleteTeam { request_id, .. }
        | WebSocketCommand::QueryTeams { request_id, .. }
        | WebSocketCommand::AddTeamMember { request_id, .. }
        | WebSocketCommand::UpdateTeamMember { request_id, .. }
        | WebSocketCommand::RemoveTeamMember { request_id, .. }
        | WebSocketCommand::ListTeamMembers { request_id, .. }
        | WebSocketCommand::InviteWorkspaceMember { request_id, .. }
        | WebSocketCommand::AcceptInvitation { request_id, .. }
        | WebSocketCommand::QueryWorkspaceMembers { request_id, .. }
        | WebSocketCommand::CreateProjectStatus { request_id, .. }
        | WebSocketCommand::UpdateProjectStatus { request_id, .. }
        | WebSocketCommand::DeleteProjectStatus { request_id, .. }
        | WebSocketCommand::QueryProjectStatuses { request_id, .. }
        | WebSocketCommand::GetProjectStatusById { request_id, .. }
        | WebSocketCommand::CreateWorkspace { request_id, .. }
        | WebSocketCommand::UpdateWorkspace { request_id, .. }
        | WebSocketCommand::DeleteWorkspace { request_id, .. }
        | WebSocketCommand::GetCurrentWorkspace { request_id, .. }
        | WebSocketCommand::UpdateProfile { request_id, .. }
        | WebSocketCommand::CreateProject { request_id, .. }
        | WebSocketCommand::UpdateProject { request_id, .. }
        | WebSocketCommand::DeleteProject { request_id, .. }
        | WebSocketCommand::QueryProjects { request_id, .. }
        | WebSocketCommand::CreateIssue { request_id, .. }
        | WebSocketCommand::UpdateIssue { request_id, .. }
        | WebSocketCommand::DeleteIssue { request_id, .. }
        | WebSocketCommand::QueryIssues { request_id, .. }
        | WebSocketCommand::GetIssue { request_id, .. } => request_id,
    };
    id.clone()
}

/// 把 enum 序列化回 JSON，cloned via serde_json。
fn to_payload(cmd: &WebSocketCommand) -> Value {
    serde_json::to_value(cmd).unwrap_or(Value::Null)
}

/// 占位 UUID（workspace_id 缺失时）
fn uuid_nil() -> uuid::Uuid {
    uuid::Uuid::nil()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::registry::handlers::{PingHandler, GetConnectionInfoHandler};
    use crate::websocket::registry::handlers::session::SubscriptionSession;
    use crate::websocket::registry::handlers::{SubscribeHandler, UnsubscribeHandler};
    use crate::websocket::subscription::SubscriptionManager;
    use chrono::{DateTime, Utc};

    fn authed_user() -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: uuid::Uuid::new_v4(),
            username: "u".into(),
            email: "u@test".into(),
            name: "u".into(),
            avatar_url: None,
            current_workspace_id: Some(uuid::Uuid::new_v4()),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registry_dispatch_ping_through_registry() {
        let reg = HandlerRegistry::new();
        reg.register(PingHandler);
        let user = authed_user();
        let cmd = WebSocketCommand::Ping { request_id: Some("r-1".into()) };
        let resp = try_dispatch(&reg, &cmd, &user).await.expect("ping should hit registry");
        assert!(resp.success);
        assert_eq!(resp.command_type, "ping");
        assert_eq!(resp.request_id.as_deref(), Some("r-1"));
        assert!(resp.data.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registry_dispatch_get_connection_info() {
        let reg = HandlerRegistry::new();
        reg.register(GetConnectionInfoHandler);
        let user = authed_user();
        let cmd = WebSocketCommand::GetConnectionInfo { request_id: None };
        let out = try_dispatch(&reg, &cmd, &user).await.expect("must hit registry");
        assert!(out.success);
        assert!(out.data.unwrap().get("user_id").is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registry_dispatch_returns_none_when_not_registered() {
        let reg = HandlerRegistry::new();
        let user = authed_user();
        let cmd = WebSocketCommand::Ping { request_id: None };
        // registry 里没有 ping handler → 由调用方走旧路径
        let out = try_dispatch(&reg, &cmd, &user).await;
        assert!(out.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registry_dispatch_subscribe_is_skiplist() {
        // 即使注册了 SubscribeHandler（用 session），registry_dispatch 也必须放行
        // 到旧路径，原因是它没有 ws connection_id 上下文。
        let mgr = Arc::new(SubscriptionManager::new());
        let session = Arc::new(SubscriptionSession::new(mgr, "c1"));
        let reg = HandlerRegistry::new();
        reg.register(SubscribeHandler::new(session.clone()));
        reg.register(UnsubscribeHandler::new(session));

        let user = authed_user();
        let cmd_sub = WebSocketCommand::Subscribe {
            topics: vec!["issues".into()],
            request_id: Some("r-sub".into()),
        };
        assert!(try_dispatch(&reg, &cmd_sub, &user).await.is_none());

        let cmd_unsub = WebSocketCommand::Unsubscribe {
            topics: vec!["issues".into()],
            request_id: Some("r-unsub".into()),
        };
        assert!(try_dispatch(&reg, &cmd_unsub, &user).await.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registry_dispatch_internal_error_wrapped_as_response_error() {
        let reg = HandlerRegistry::new();
        // 注册一个会失败的 handler
        struct FailHandler;
        #[async_trait::async_trait]
        impl crate::websocket::registry::CommandHandler for FailHandler {
            fn command_type(&self) -> &'static str { "ping" }
            async fn handle(
                &self,
                _ctx: RequestContext,
                _payload: serde_json::Value,
            ) -> Result<serde_json::Value, crate::websocket::registry::HandlerError> {
                Err(crate::websocket::registry::HandlerError::Internal {
                    detail: "boom".into(),
                })
            }
        }
        reg.register(FailHandler);
        let user = authed_user();
        let cmd = WebSocketCommand::Ping { request_id: Some("r".into()) };
        let resp = try_dispatch(&reg, &cmd, &user).await.unwrap();
        assert!(!resp.success);
        assert_eq!(resp.command_type, "ping");
        let err = resp.error.unwrap();
        assert_eq!(err.code, "SYSTEM_ERROR");
        assert!(err.message.contains("boom"));
    }

    // Suppress unused
    #[allow(dead_code)]
    fn _suppress(_: DateTime<Utc>) {}
}
