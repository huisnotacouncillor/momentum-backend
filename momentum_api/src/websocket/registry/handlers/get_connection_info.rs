//! GetConnectionInfoHandler — 最小示例，验证 registry trait 可用
//!
//! 不依赖 DB / manager；返回基于 RequestContext 的"连接"快照。
//! 真实实现要在 Step 6+ 接入 `WebSocketManager`。

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};

use momentum_core::services::context::RequestContext;

use crate::websocket::registry::{CommandHandler, HandlerConfig, HandlerError};

pub struct GetConnectionInfoHandler;

impl GetConnectionInfoHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetConnectionInfoHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for GetConnectionInfoHandler {
    fn command_type(&self) -> &'static str {
        "get_connection_info"
    }

    fn config(&self) -> HandlerConfig {
        HandlerConfig {
            timeout_ms: Some(2_000),
        }
    }

    async fn handle(
        &self,
        ctx: RequestContext,
        _payload: Value,
    ) -> Result<Value, HandlerError> {
        Ok(json!({
            "user_id": ctx.user_id,
            "workspace_id": ctx.workspace_id,
            "idempotency_key": ctx.idempotency_key,
            "ts": Utc::now(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::registry::HandlerRegistry;
    use uuid::Uuid;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_connection_info_returns_context_snapshot() {
        let reg = HandlerRegistry::new();
        reg.register(GetConnectionInfoHandler);
        let ctx = RequestContext {
            user_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            idempotency_key: Some("v1.0".into()),
            trace_id: "test".into(),
        };
        let out = reg.dispatch("get_connection_info", ctx, json!({})).unwrap();
        assert!(out["user_id"].is_string());
        assert!(out["workspace_id"].is_string());
        assert_eq!(out["idempotency_key"], "v1.0");
    }
}
