//! SubscribeHandler — 注册到 registry，桥接 `subscribe` 命令到 SubscriptionManager

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use momentum_core::services::context::RequestContext;

use crate::websocket::registry::{CommandHandler, HandlerConfig, HandlerError};
use crate::websocket::subscription::Topic;

use super::session::SubscriptionSession;

pub struct SubscribeHandler {
    session: Arc<SubscriptionSession>,
}

impl SubscribeHandler {
    pub fn new(session: Arc<SubscriptionSession>) -> Self {
        Self { session }
    }
}

#[async_trait]
impl CommandHandler for SubscribeHandler {
    fn command_type(&self) -> &'static str {
        "subscribe"
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
                detail: "subscribe: missing 'topics' array".into(),
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
                    "subscribe: no valid topics, errors={}",
                    serde_json::to_string(&errors).unwrap_or_default()
                ),
            });
        }

        let result = self.session.subscribe(&parsed).await;

        Ok(json!({
            "subscribed": result
                .subscribed
                .iter()
                .map(|t| t.as_string())
                .collect::<Vec<_>>(),
            "duplicates": result
                .duplicates
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscribe_three_topics_returns_subscribed_list() {
        let session = make_session();
        let reg = HandlerRegistry::new();
        reg.register(SubscribeHandler::new(session.clone()));

        let out = reg
            .dispatch(
                "subscribe",
                ctx(),
                json!({ "topics": ["issues", "projects", "issues:*:created"] }),
            )
            .unwrap();
        let subscribed = out["subscribed"].as_array().unwrap();
        assert_eq!(subscribed.len(), 3);
        let errors = out["errors"].as_array().unwrap();
        assert!(errors.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscribe_invalid_topic_yields_error_in_response() {
        let session = make_session();
        let reg = HandlerRegistry::new();
        reg.register(SubscribeHandler::new(session.clone()));

        let out = reg
            .dispatch(
                "subscribe",
                ctx(),
                json!({ "topics": ["issues", "with space"] }),
            )
            .unwrap();
        // "issues" 成功，"with space" 失败
        let subscribed = out["subscribed"].as_array().unwrap();
        assert_eq!(subscribed.len(), 1);
        assert_eq!(subscribed[0], "issues");
        let errors = out["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["input"], "with space");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscribe_missing_topics_field_is_internal_error() {
        let reg = HandlerRegistry::new();
        reg.register(SubscribeHandler::new(make_session()));
        let r = reg.dispatch("subscribe", ctx(), json!({}));
        match r {
            Err(HandlerError::Internal { detail }) => assert!(detail.contains("topics")),
            other => panic!("expected Internal, got {:?}", other),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscribe_all_invalid_topics_rejected() {
        let reg = HandlerRegistry::new();
        reg.register(SubscribeHandler::new(make_session()));
        let r = reg.dispatch(
            "subscribe",
            ctx(),
            json!({ "topics": ["bad topic", "another bad"] }),
        );
        match r {
            Err(HandlerError::Internal { detail }) => {
                assert!(detail.contains("no valid topics"))
            }
            other => panic!("expected Internal, got {:?}", other),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn duplicate_subscribe_reflected_in_duplicates_field() {
        let session = make_session();
        let reg = HandlerRegistry::new();
        reg.register(SubscribeHandler::new(session.clone()));

        reg.dispatch("subscribe", ctx(), json!({ "topics": ["issues"] }))
            .unwrap();
        let out2 = reg
            .dispatch("subscribe", ctx(), json!({ "topics": ["issues"] }))
            .unwrap();
        // 第二次：subscribed 是空、duplicates 是 ["issues"]
        assert!(out2["subscribed"].as_array().unwrap().is_empty());
        assert_eq!(out2["duplicates"].as_array().unwrap().len(), 1);
    }
}
