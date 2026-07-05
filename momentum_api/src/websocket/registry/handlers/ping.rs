//! PingHandler — 最小示例，验证 registry trait 可用
//!
//! 不依赖 DB；直接返回 `{ ok: true, echo: <payload>, ts: <utc> }`。
//! 后续 Step 7+ 接入真实业务 handler 时复用此模板。

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};

use momentum_core::services::context::RequestContext;

use crate::websocket::registry::{CommandHandler, HandlerConfig, HandlerError};

pub struct PingHandler;

impl PingHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PingHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for PingHandler {
    fn command_type(&self) -> &'static str {
        "ping"
    }

    fn config(&self) -> HandlerConfig {
        HandlerConfig {
            timeout_ms: Some(2_000),
        }
    }

    async fn handle(
        &self,
        ctx: RequestContext,
        payload: Value,
    ) -> Result<Value, HandlerError> {
        Ok(json!({
            "ok": true,
            "echo": payload,
            "user_id": ctx.user_id,
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
    async fn ping_handler_registers_and_dispatches() {
        let reg = HandlerRegistry::new();
        reg.register(PingHandler);
        let ctx = RequestContext {
            user_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            idempotency_key: None,
        trace_id: "unknown".to_string(),
        };
        let out = reg
            .dispatch("ping", ctx, json!({ "hi": "there" }))
            .unwrap();
        assert_eq!(out["ok"], true);
        assert_eq!(out["echo"]["hi"], "there");
        assert!(out["ts"].is_string());
    }
}
