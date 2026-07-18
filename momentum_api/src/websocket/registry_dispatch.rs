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
//! - subscribe/unsubscribe 通过 SubscriptionSession 接入（Step 8.5）

use std::sync::Arc;

use serde_json::Value;

use momentum_core::services::context::RequestContext;

use crate::websocket::auth::AuthenticatedUser;
use crate::websocket::commands::types::{
    WebSocketCommand, WebSocketCommandError, WebSocketCommandResponse,
};
use crate::websocket::registry::{HandlerError, HandlerRegistry};
use crate::websocket::registry::handlers::{SubscribeHandler, UnsubscribeHandler};
use crate::websocket::registry::handlers::SubscriptionSession;
use crate::websocket::manager::subscription::SubscriptionManager;

/// 优先尝试 registry；如果 registry 没有该命令，返回 None。
///
/// Step 8.5: subscribe/unsubscribe are handled when a SubscriptionManager is provided.
/// A per-connection SubscriptionSession is created and the handlers are registered
/// on-the-fly so they have the correct connection_id.
///
/// 调用方应该这样使用：
/// ```ignore
/// pub async fn handle_command(&self, command, user, connection_id) -> WebSocketCommandResponse {
///     if let Some(resp) = registry_dispatch::try_dispatch(
///         &self.registry, self.subscription_manager.as_ref(),
///         &command, user, connection_id
///     ).await {
///         return resp;
///     }
///     self.handle_command_legacy(command, user).await
/// }
/// ```
pub async fn try_dispatch(
    reg: &HandlerRegistry,
    sub_mgr: Option<&Arc<SubscriptionManager>>,
    command: &WebSocketCommand,
    user: &AuthenticatedUser,
    connection_id: &str,
) -> Option<WebSocketCommandResponse> {
    let command_type = command.command_type();

    // Step 8.5: subscribe/unsubscribe need per-connection SubscriptionSession.
    // When a SubscriptionManager is provided, register handlers dynamically with
    // a session bound to this connection_id.
    if command_type == "subscribe" || command_type == "unsubscribe" {
        if let Some(mgr) = sub_mgr {
            let session = Arc::new(SubscriptionSession::new(mgr.clone(), connection_id));
            // Register on-the-fly (Arc clones, no DB calls — cheap)
            reg.register(SubscribeHandler::new(session.clone()));
            reg.register(UnsubscribeHandler::new(session));
        } else {
            // No SubscriptionManager — fall through to legacy handler
            return None;
        }
    }

    if !reg.registered_types().iter().any(|t| *t == command_type) {
        return None;
    }

    let payload = serde_json::to_value(command).unwrap_or(Value::Null);
    let ctx = RequestContext {
        user_id: user.user_id,
        workspace_id: user.current_workspace_id.unwrap_or_else(uuid_nil),
        idempotency_key: None,
        trace_id: "unknown".to_string(),
    };
    let request_id = command.request_id();

    match reg.dispatch(command_type, ctx, payload) {
        Ok(value) => Some(WebSocketCommandResponse::success(
            command_type,
            "registry",
            request_id,
            value,
        )),
        Err(HandlerError::NotFound { command_type }) => {
            // In theory shouldn't happen since we just checked registered_types.
            // Defensive fallback to legacy path.
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

/// 占位 UUID（workspace_id 缺失时）
fn uuid_nil() -> uuid::Uuid {
    uuid::Uuid::nil()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::registry::handlers::{PingHandler, GetConnectionInfoHandler};
    use crate::websocket::manager::subscription::SubscriptionManager;
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
        let resp = try_dispatch(&reg, None, &cmd, &user, "c1")
            .await
            .expect("ping should hit registry");
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
        let out = try_dispatch(&reg, None, &cmd, &user, "c1")
            .await
            .expect("must hit registry");
        assert!(out.success);
        assert!(out.data.unwrap().get("user_id").is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registry_dispatch_returns_none_when_not_registered() {
        let reg = HandlerRegistry::new();
        let user = authed_user();
        let cmd = WebSocketCommand::Ping { request_id: None };
        // registry 里没有 ping handler → 由调用方走旧路径
        let out = try_dispatch(&reg, None, &cmd, &user, "c1").await;
        assert!(out.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registry_dispatch_subscribe_via_subscription_manager() {
        // Step 8.5: when SubscriptionManager is provided, subscribe IS handled via registry
        let mgr = Arc::new(SubscriptionManager::new());
        let reg = HandlerRegistry::new();

        let user = authed_user();
        let cmd_sub = WebSocketCommand::Subscribe {
            topics: vec!["issues".into()],
            request_id: Some("r-sub".into()),
        };
        let resp = try_dispatch(&reg, Some(&mgr), &cmd_sub, &user, "c1")
            .await
            .expect("subscribe should be handled when sub_mgr provided");
        assert!(resp.success);
        assert_eq!(resp.command_type, "subscribe");

        let cmd_unsub = WebSocketCommand::Unsubscribe {
            topics: vec!["issues".into()],
            request_id: Some("r-unsub".into()),
        };
        let resp_unsub = try_dispatch(&reg, Some(&mgr), &cmd_unsub, &user, "c1")
            .await
            .expect("unsubscribe should be handled when sub_mgr provided");
        assert!(resp_unsub.success);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registry_dispatch_subscribe_without_sub_mgr_falls_back() {
        // Without SubscriptionManager, subscribe falls through to legacy handler (returns None)
        let reg = HandlerRegistry::new();
        let user = authed_user();
        let cmd_sub = WebSocketCommand::Subscribe {
            topics: vec!["issues".into()],
            request_id: Some("r-sub".into()),
        };
        let out = try_dispatch(&reg, None, &cmd_sub, &user, "c1").await;
        assert!(out.is_none(), "should return None when no SubscriptionManager for subscribe");
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
        let resp = try_dispatch(&reg, None, &cmd, &user, "c1").await.unwrap();
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
