//! UnsubscribeHandler — 反向操作

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use momentum_core::services::context::RequestContext;

use crate::websocket::registry::{CommandHandler, HandlerConfig, HandlerError};
use crate::websocket::subscription::Topic;

use super::session::SubscriptionSession;

pub struct UnsubscribeHandler {
    session: Arc<SubscriptionSession>,
}

impl UnsubscribeHandler {
    pub fn new(session: Arc<SubscriptionSession>) -> Self {
        Self { session }
    }
}

#[async_trait]
impl CommandHandler for UnsubscribeHandler {
    fn command_type(&self) -> &'static str {
        "unsubscribe"
    }

    fn config(&self) -> HandlerConfig {
        HandlerConfig {
            timeout_ms: Some(5_000),
        }
    }

    async fn handle(
        &self,
        _ctx: RequestContext,
        payload: Value,
    ) -> Result<Value, HandlerError> {
        let raw = payload
            .get("topics")
            .and_then(|v| v.as_array())
            .ok_or_else(|| HandlerError::Internal {
                detail: "unsubscribe: missing 'topics' array".into(),
            })?;

        let mut parsed = Vec::new();
        let mut errors = Vec::new();
        for v in raw {
            if let Some(s) = v.as_str() {
                match Topic::parse(s) {
                    Ok(t) => parsed.push(t),
                    Err(e) => errors.push(json!({ "input": s, "error": e.to_string() })),
                }
            } else {
                errors.push(json!({ "input": "non-string", "error": "topic must be string" }));
            }
        }

        if parsed.is_empty() {
            return Err(HandlerError::Internal {
                detail: format!(
                    "unsubscribe: no valid topics, errors={}",
                    serde_json::to_string(&errors).unwrap_or_default()
                ),
            });
        }

        let result = self.session.unsubscribe(&parsed).await;

        Ok(json!({
            "unsubscribed": result
                .unsubscribed
                .iter()
                .map(|t| t.as_string())
                .collect::<Vec<_>>(),
            "errors": errors,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::registry::HandlerRegistry;
    use crate::websocket::subscription::SubscriptionManager;

    fn make_session() -> Arc<SubscriptionSession> {
        let mgr = Arc::new(SubscriptionManager::new());
        Arc::new(SubscriptionSession::new(mgr, "test-c1"))
    }

    fn ctx() -> RequestContext {
        RequestContext {
            user_id: uuid::Uuid::new_v4(),
            workspace_id: uuid::Uuid::new_v4(),
            idempotency_key: None,
        trace_id: "unknown".to_string(),
        }
    }

    /// 先通过 manager 直接 subscribe，再用 UnsubscribeHandler 取消
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unsubscribe_returns_unsubscribed_list() {
        let session = make_session();
        // 先放入订阅（直接走 manager，模拟别处发起的）
        session
            .manager()
            .subscribe("test-c1", &[Topic::parse("issues").unwrap()])
            .await;

        let reg = HandlerRegistry::new();
        reg.register(UnsubscribeHandler::new(session.clone()));
        let out = reg
            .dispatch("unsubscribe", ctx(), json!({ "topics": ["issues"] }))
            .unwrap();
        assert_eq!(out["unsubscribed"].as_array().unwrap().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unsubscribe_missing_topics_is_internal_error() {
        let reg = HandlerRegistry::new();
        reg.register(UnsubscribeHandler::new(make_session()));
        let r = reg.dispatch("unsubscribe", ctx(), json!({}));
        match r {
            Err(HandlerError::Internal { detail }) => assert!(detail.contains("topics")),
            other => panic!("expected Internal, got {:?}", other),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unsubscribe_unknown_topic_returns_empty() {
        let reg = HandlerRegistry::new();
        reg.register(UnsubscribeHandler::new(make_session()));
        // 没订阅过 -> unsubscribed 是空数组（不是错误）
        let out = reg
            .dispatch("unsubscribe", ctx(), json!({ "topics": ["issues"] }))
            .unwrap();
        assert_eq!(out["unsubscribed"].as_array().unwrap().len(), 0);
    }
}
