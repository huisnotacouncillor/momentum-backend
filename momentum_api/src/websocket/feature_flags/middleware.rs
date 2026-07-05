//! FeatureFlagMiddleware (spec §4 + §5)
//!
//! 中间件层检查：`flags.is_command_enabled(envelope.command_type)`。
//! 关闭的命令直接返回 `AppError::Internal`（FEATURE_FLAG_DISABLED 语义）；
//! 启用则交给 next。
//!
//! 此中间件应当放在 chain 的"前段"，早于 metrics / rate-limit。
//! 它读 `MiddlewareContext::feature_flags`，所以与 `MiddlewareChain`
//! 当前已经持有的字段天然契合——不需要扩展 `MiddlewareContext`。

use async_trait::async_trait;
use serde_json::Value;

use momentum_core::error::AppError;

use crate::websocket::middleware::{
    CommandEnvelope, CommandMiddleware, MiddlewareContext, NextMiddleware,
};

pub struct FeatureFlagMiddleware;

impl FeatureFlagMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FeatureFlagMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandMiddleware for FeatureFlagMiddleware {
    fn name(&self) -> &'static str {
        "feature_flag"
    }

    async fn process(
        &self,
        envelope: CommandEnvelope,
        ctx: &MiddlewareContext,
        next: NextMiddleware<'_>,
    ) -> Result<Value, AppError> {
        if ctx.feature_flags.is_command_enabled(envelope.command_type) {
            next.run().await
        } else {
            Err(AppError::Internal(format!(
                "FEATURE_FLAG_DISABLED: command '{}' is not enabled",
                envelope.command_type
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::feature_flags::FeatureFlags;
    use crate::websocket::middleware::MiddlewareChain;
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;
    use momentum_core::services::context::RequestContext;

    fn make_env(command_type: &'static str, payload: Value) -> CommandEnvelope {
        CommandEnvelope::new(
            command_type,
            payload,
            RequestContext {
                user_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                idempotency_key: None,
        trace_id: "unknown".to_string(),
            },
            Some("req-test".into()),
        )
    }

    fn make_ctx(flags: Arc<FeatureFlags>) -> MiddlewareContext {
        MiddlewareContext {
            feature_flags: flags,
        }
    }

    #[tokio::test]
    async fn enabled_command_passes_through() {
        let flags = Arc::new(FeatureFlags::default());
        let ctx = make_ctx(flags);
        // ping 在默认值里始终 enable
        let env = make_env("ping", json!({}));
        let chain = MiddlewareChain::new().push(FeatureFlagMiddleware::new());
        let out = chain.execute(env, &ctx).await.unwrap();
        // 终端：原样返回 payload
        assert_eq!(out, json!({}));
    }

    #[tokio::test]
    async fn disabled_command_is_blocked() {
        let flags = Arc::new(FeatureFlags::default());
        let ctx = make_ctx(flags);
        // create_issue 在默认值里 disable
        let env = make_env("create_issue", json!({ "title": "x" }));
        let chain = MiddlewareChain::new().push(FeatureFlagMiddleware::new());
        let err = chain.execute(env, &ctx).await.unwrap_err();
        match err {
            AppError::Internal(msg) => {
                assert!(msg.contains("FEATURE_FLAG_DISABLED"));
                assert!(msg.contains("create_issue"));
            }
            other => panic!("expected AppError::Internal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn toggling_flags_changes_outcome() {
        let mut flags = FeatureFlags::default();
        flags.issue_create_enabled = true;
        let flags = Arc::new(flags);
        let ctx = make_ctx(flags);
        let env = make_env("create_issue", json!({}));
        let chain = MiddlewareChain::new().push(FeatureFlagMiddleware::new());
        // 启用后能通过
        let out = chain.execute(env, &ctx).await.unwrap();
        assert_eq!(out, json!({}));
    }

    #[tokio::test]
    async fn middleware_short_circuits_chain() {
        // 让 feature flag 在第一位，下面假装有一个会改 payload 的 middleware，
        // 验证 create_issue 被拦下、不进入下游。
        use async_trait::async_trait;
        struct BadDownstream;
        #[async_trait]
        impl CommandMiddleware for BadDownstream {
            fn name(&self) -> &'static str { "bad" }
            async fn process(
                &self,
                _env: CommandEnvelope,
                _ctx: &MiddlewareContext,
                _next: NextMiddleware<'_>,
            ) -> Result<Value, AppError> {
                Err(AppError::Internal("should never run".into()))
            }
        }

        let flags = Arc::new(FeatureFlags::default());
        let ctx = make_ctx(flags);
        let env = make_env("create_issue", json!({}));
        let chain = MiddlewareChain::new()
            .push(FeatureFlagMiddleware::new())
            .push(BadDownstream);
        // 拦下：产生 AppError::Internal 而不是 BadDownstream 的
        let err = chain.execute(env, &ctx).await.unwrap_err();
        match err {
            AppError::Internal(msg) => assert!(msg.contains("FEATURE_FLAG_DISABLED")),
            other => panic!("unexpected {:?}", other),
        }
    }
}
