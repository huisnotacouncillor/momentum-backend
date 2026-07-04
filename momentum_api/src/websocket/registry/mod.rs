//! Handler registry (spec §3) — 最小骨架
//!
//! 这一步**只**引入 `CommandHandler` trait 与 `HandlerRegistry` 的接口；
//! 真正把 `WebSocketCommandHandler` 中的方法桥接到 Registry 留到 Step 2。
//!
//! 设计选择：
//! - 命令类型作为 `&'static str` 索引（与现有 enum variant 同名，snake_case）；
//! - handler 通过 `Arc<dyn CommandHandler>` 持有，trait 的方法 `async`；
//! - 注册与查表均 `Send + Sync`，方便从 axum 的 State 直接 `Arc::clone`。
//!
//! 不删除任何现有 `commands/` 模块。

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

use momentum_core::error::AppError;
use momentum_core::services::context::RequestContext;

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Command handler not found: {command_type}")]
    NotFound { command_type: &'static str },

    #[error("Feature disabled: {feature}")]
    FeatureDisabled { feature: &'static str },

    #[error("Internal error: {detail}")]
    Internal { detail: String },
}

impl From<HandlerError> for AppError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::NotFound { command_type } => {
                AppError::NotFound { resource: format!("handler for {command_type}") }
            }
            HandlerError::FeatureDisabled { feature } => {
                AppError::Internal(format!("feature disabled: {feature}"))
            }
            HandlerError::Internal { detail } => AppError::Internal(detail),
        }
    }
}

/// Handler 配置（每命令的 timeout / rate-limit）。Step 1 仅占位，未与
/// 现有 `RetryTimeoutManager` 集成。
#[derive(Debug, Clone, Default)]
pub struct HandlerConfig {
    pub timeout_ms: Option<u64>,
}

#[async_trait]
pub trait CommandHandler: Send + Sync {
    fn command_type(&self) -> &'static str;
    fn config(&self) -> HandlerConfig {
        HandlerConfig::default()
    }
    async fn handle(
        &self,
        ctx: RequestContext,
        payload: Value,
    ) -> Result<Value, HandlerError>;
}

#[derive(Default)]
pub struct HandlerRegistry {
    inner: RwLock<HashMap<&'static str, Arc<dyn CommandHandler>>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<H: CommandHandler + 'static>(&self, handler: H) {
        let key = handler.command_type();
        if let Ok(mut map) = self.inner.write() {
            map.insert(key, Arc::new(handler));
        }
    }

    pub fn dispatch(
        &self,
        command_type: &'static str,
        ctx: RequestContext,
        payload: Value,
    ) -> Result<Value, HandlerError> {
        let handler = {
            let map = self.inner.read().map_err(|_| HandlerError::Internal {
                detail: "registry poisoned".into(),
            })?;
            map.get(command_type).cloned()
        };
        match handler {
            Some(h) => {
                // Step 1：blocking call；Step 3 接入 tokio::spawn
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(h.handle(ctx, payload))
                })
            }
            None => Err(HandlerError::NotFound { command_type }),
        }
    }

    pub fn registered_types(&self) -> Vec<&'static str> {
        self.inner
            .read()
            .map(|m| m.keys().copied().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    struct EchoHandler;
    #[async_trait]
    impl CommandHandler for EchoHandler {
        fn command_type(&self) -> &'static str { "echo" }
        async fn handle(
            &self,
            ctx: RequestContext,
            payload: Value,
        ) -> Result<Value, HandlerError> {
            Ok(json!({ "echo": payload, "user": ctx.user_id }))
        }
    }

    fn ctx() -> RequestContext {
        RequestContext {
            user_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            idempotency_key: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn register_and_dispatch_roundtrip() {
        let reg = HandlerRegistry::new();
        reg.register(EchoHandler);
        let out = reg
            .dispatch("echo", ctx(), json!({ "hello": "world" }))
            .unwrap();
        assert_eq!(out["echo"]["hello"], "world");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dispatch_unknown_returns_not_found() {
        let reg = HandlerRegistry::new();
        let r = reg.dispatch("missing", ctx(), json!({}));
        assert!(matches!(r, Err(HandlerError::NotFound { .. })));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registered_types_lists_keys() {
        let reg = HandlerRegistry::new();
        reg.register(EchoHandler);
        // 二次注册同 key 不报错，覆盖
        reg.register(EchoHandler);
        let types = reg.registered_types();
        assert_eq!(types, vec!["echo"]);
    }

    #[test]
    fn handler_error_into_apperror() {
        let e = HandlerError::NotFound { command_type: "x" };
        let _: AppError = e.into();
        let e = HandlerError::FeatureDisabled { feature: "y" };
        let _: AppError = e.into();
        let e = HandlerError::Internal { detail: "z".into() };
        let _: AppError = e.into();
    }
}
