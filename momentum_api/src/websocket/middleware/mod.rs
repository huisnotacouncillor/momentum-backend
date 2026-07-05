//! middleware module 索引

pub mod metrics;

pub use metrics::{MetricsEvent, MetricsMiddleware, MetricsSink, TracingSink};

use async_trait::async_trait;
use serde_json::Value;

use momentum_core::error::AppError;
use momentum_core::services::context::RequestContext;

// ===== 既有 trait/chain/envelope 代码 =====

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CommandEnvelope {
    pub command_type: &'static str,
    pub payload: Value,
    pub context: RequestContext,
    pub request_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl CommandEnvelope {
    pub fn new(
        command_type: &'static str,
        payload: Value,
        context: RequestContext,
        request_id: Option<String>,
    ) -> Self {
        Self {
            command_type,
            payload,
            context,
            request_id,
            metadata: HashMap::new(),
        }
    }
}

#[async_trait]
pub trait CommandMiddleware: Send + Sync {
    fn name(&self) -> &'static str;
    async fn process(
        &self,
        envelope: CommandEnvelope,
        ctx: &MiddlewareContext,
        next: NextMiddleware<'_>,
    ) -> Result<Value, AppError>;
}

#[derive(Clone)]
pub struct MiddlewareContext {
    pub feature_flags: std::sync::Arc<crate::websocket::feature_flags::FeatureFlags>,
}

pub struct NextMiddleware<'a> {
    chain: &'a [Box<dyn CommandMiddleware>],
    index: usize,
    envelope: CommandEnvelope,
    ctx: &'a MiddlewareContext,
}

impl<'a> NextMiddleware<'a> {
    pub async fn run(self) -> Result<Value, AppError> {
        if self.index >= self.chain.len() {
            return Ok(self.envelope.payload);
        }
        let mw = &self.chain[self.index];
        let envelope = self.envelope.clone();
        let next = NextMiddleware {
            chain: self.chain,
            index: self.index + 1,
            envelope,
            ctx: self.ctx,
        };
        mw.process(next.envelope.clone(), self.ctx, next).await
    }
}

pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn CommandMiddleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self { middlewares: Vec::new() }
    }

    pub fn push<M: CommandMiddleware + 'static>(mut self, mw: M) -> Self {
        self.middlewares.push(Box::new(mw));
        self
    }

    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    pub async fn execute(
        &self,
        envelope: CommandEnvelope,
        ctx: &MiddlewareContext,
    ) -> Result<Value, AppError> {
        let next = NextMiddleware {
            chain: &self.middlewares,
            index: 0,
            envelope,
            ctx,
        };
        next.run().await
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use uuid::Uuid;

    fn make_ctx() -> (CommandEnvelope, MiddlewareContext) {
        let env = CommandEnvelope::new(
            "ping",
            json!({}),
            RequestContext {
                user_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                idempotency_key: None,
        trace_id: "unknown".to_string(),
            },
            Some("req-1".into()),
        );
        let ctx = MiddlewareContext {
            feature_flags: std::sync::Arc::new(
                crate::websocket::feature_flags::FeatureFlags::default(),
            ),
        };
        (env, ctx)
    }

    struct Wrapper(String);
    #[async_trait]
    impl CommandMiddleware for Wrapper {
        fn name(&self) -> &'static str { "wrapper" }
        async fn process(
            &self,
            _envelope: CommandEnvelope,
            _ctx: &MiddlewareContext,
            next: NextMiddleware<'_>,
        ) -> Result<Value, AppError> {
            let inner = next.run().await?;
            Ok(json!({ "by": self.0, "inner": inner }))
        }
    }

    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    fn reset() { COUNTER.store(0, Ordering::SeqCst); }
    fn next_seq() -> usize { COUNTER.fetch_add(1, Ordering::SeqCst) }

    struct SequenceRecorder(&'static str);
    #[async_trait]
    impl CommandMiddleware for SequenceRecorder {
        fn name(&self) -> &'static str { "seq" }
        async fn process(
            &self,
            _envelope: CommandEnvelope,
            _ctx: &MiddlewareContext,
            next: NextMiddleware<'_>,
        ) -> Result<Value, AppError> {
            let pre_seq = next_seq();
            let name = self.0;
            let mut out = next.run().await?;
            let post_seq = next_seq();
            let obj = out.as_object_mut().unwrap();
            let visits = obj
                .entry("__visits".to_string())
                .or_insert_with(|| json!([]));
            visits.as_array_mut().unwrap().push(json!({
                "name": name,
                "pre_seq": pre_seq,
                "post_seq": post_seq,
            }));
            Ok(out)
        }
    }

    #[tokio::test]
    async fn empty_chain_passes_payload_through() {
        let (env, ctx) = make_ctx();
        let chain = MiddlewareChain::new();
        let out = chain.execute(env, &ctx).await.unwrap();
        assert_eq!(out, json!({}));
    }

    #[tokio::test]
    async fn single_wrapper_wraps_terminal() {
        let (env, ctx) = make_ctx();
        let chain = MiddlewareChain::new().push(Wrapper("a".into()));
        let out = chain.execute(env, &ctx).await.unwrap();
        assert_eq!(out["by"], "a");
        assert_eq!(out["inner"], json!({}));
    }

    #[tokio::test]
    async fn wrapper_chains_outside_in() {
        let (env, ctx) = make_ctx();
        let chain = MiddlewareChain::new()
            .push(Wrapper("a".into()))
            .push(Wrapper("b".into()));
        let out = chain.execute(env, &ctx).await.unwrap();
        assert_eq!(out["by"], "a");
        assert_eq!(out["inner"]["by"], "b");
        assert_eq!(out["inner"]["inner"], json!({}));
    }

    #[tokio::test]
    async fn visit_order_is_fifo_pre_lifo_post() {
        reset();
        let (env, ctx) = make_ctx();
        let chain = MiddlewareChain::new()
            .push(SequenceRecorder("a"))
            .push(SequenceRecorder("b"))
            .push(SequenceRecorder("c"));
        let out = chain.execute(env, &ctx).await.unwrap();
        let v = out["__visits"].as_array().unwrap();
        let pre: Vec<_> = v
            .iter()
            .map(|x| x["pre_seq"].as_u64().unwrap() as usize)
            .collect();
        let names: Vec<&str> = v.iter().map(|x| x["name"].as_str().unwrap()).collect();
        assert_eq!(pre, vec![2, 1, 0]);
        assert_eq!(names, vec!["c", "b", "a"]);
        let post: Vec<_> = v
            .iter()
            .map(|x| x["post_seq"].as_u64().unwrap() as usize)
            .collect();
        assert_eq!(post, vec![3, 4, 5]);
    }

    // ===== 集成测试（与 feature_flag / version 中间件串联） =====

    #[tokio::test]
    async fn integration_ping_passes_through_full_chain() {
        use crate::websocket::feature_flags::FeatureFlagMiddleware;
        use crate::websocket::protocol::VersionNegotiationMiddleware;
        use std::sync::Arc;

        let flags = Arc::new(crate::websocket::feature_flags::FeatureFlags::default());
        let ctx = MiddlewareContext { feature_flags: flags };
        let env = CommandEnvelope::new(
            "ping",
            json!({}),
            RequestContext {
                user_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                idempotency_key: None,
        trace_id: "unknown".to_string(),
            },
            Some("req-it-1".into()),
        );
        let chain = MiddlewareChain::new()
            .push(FeatureFlagMiddleware::new())
            .push(VersionNegotiationMiddleware::new());
        let out = chain.execute(env, &ctx).await.unwrap();
        assert_eq!(out, json!({}));
    }

    #[tokio::test]
    async fn integration_create_issue_blocked_by_feature_flag() {
        use crate::websocket::feature_flags::FeatureFlagMiddleware;
        use std::sync::Arc;

        let flags = Arc::new(crate::websocket::feature_flags::FeatureFlags::default());
        let ctx = MiddlewareContext { feature_flags: flags };
        let env = CommandEnvelope::new(
            "create_issue",
            json!({"title": "x"}),
            RequestContext {
                user_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                idempotency_key: None,
        trace_id: "unknown".to_string(),
            },
            Some("req-it-2".into()),
        );
        let chain = MiddlewareChain::new().push(FeatureFlagMiddleware::new());
        let err = chain.execute(env, &ctx).await.unwrap_err();
        match err {
            AppError::Internal(m) => {
                assert!(m.contains("FEATURE_FLAG_DISABLED"));
                assert!(m.contains("create_issue"));
            }
            other => panic!("expected Internal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn integration_chain_order_matters() {
        use crate::websocket::feature_flags::FeatureFlagMiddleware;
        use crate::websocket::protocol::VersionNegotiationMiddleware;
        use std::sync::Arc;

        let flags = Arc::new(crate::websocket::feature_flags::FeatureFlags::default());
        let ctx = MiddlewareContext { feature_flags: flags };
        let mut env = CommandEnvelope::new(
            "create_issue",
            json!({}),
            RequestContext {
                user_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                idempotency_key: None,
        trace_id: "unknown".to_string(),
            },
            None,
        );
        env.metadata.insert("ws_version".to_string(), "9.9".to_string());

        let chain1 = MiddlewareChain::new()
            .push(FeatureFlagMiddleware::new())
            .push(VersionNegotiationMiddleware::new());
        let err1 = chain1.execute(env.clone(), &ctx).await.unwrap_err();
        match err1 {
            AppError::Internal(m) => assert!(m.contains("FEATURE_FLAG_DISABLED")),
            other => panic!("chain1 expected FF, got {:?}", other),
        }

        let chain2 = MiddlewareChain::new()
            .push(VersionNegotiationMiddleware::new())
            .push(FeatureFlagMiddleware::new());
        let err2 = chain2.execute(env, &ctx).await.unwrap_err();
        match err2 {
            AppError::Internal(m) => {
                assert!(m.contains("UNSUPPORTED_VERSION"), "got: {m}");
            }
            other => panic!("chain2 expected version, got {:?}", other),
        }
    }
}
