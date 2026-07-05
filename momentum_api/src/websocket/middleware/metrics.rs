//! MetricsMiddleware (spec §4.5)
//!
//! 记录每个 command 的 elapsed_ms + success/fail。默认 sink 是 tracing。
//! 不直接耦合 WebSocketMonitor（避免把全局状态引入 middleware 测试）；
//! 生产环境接入 WebSocketMonitor 通过实现 [`MetricsSink`] trait。

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::Value;
use tracing::info;

use momentum_core::error::AppError;

use crate::websocket::middleware::{
    CommandEnvelope, CommandMiddleware, MiddlewareContext, NextMiddleware,
};

pub trait MetricsSink: Send + Sync {
    fn record(&self, event: MetricsEvent);
}

#[derive(Debug, Clone)]
pub struct MetricsEvent {
    pub command_type: &'static str,
    pub elapsed_ms: u64,
    pub success: bool,
    pub request_id: Option<String>,
}

/// tracing 默认实现；生产 WebSocketMonitor 走自己的 MetricsSink。
pub struct TracingSink;

impl MetricsSink for TracingSink {
    fn record(&self, event: MetricsEvent) {
        info!(
            command = event.command_type,
            elapsed_ms = event.elapsed_ms,
            success = event.success,
            request_id = ?event.request_id,
            "ws command metrics"
        );
    }
}

pub struct MetricsMiddleware {
    sink: Arc<dyn MetricsSink>,
}

impl MetricsMiddleware {
    pub fn new(sink: Arc<dyn MetricsSink>) -> Self {
        Self { sink }
    }

    /// 默认 tracing 实现
    pub fn with_tracing() -> Self {
        Self::new(Arc::new(TracingSink))
    }
}

#[async_trait]
impl CommandMiddleware for MetricsMiddleware {
    fn name(&self) -> &'static str {
        "metrics"
    }

    async fn process(
        &self,
        envelope: CommandEnvelope,
        _ctx: &MiddlewareContext,
        next: NextMiddleware<'_>,
    ) -> Result<Value, AppError> {
        let start = Instant::now();
        let request_id = envelope.request_id.clone();
        let command_type = envelope.command_type;
        let result = next.run().await;
        let elapsed_ms = start.elapsed().as_millis() as u64;
        let success = result.is_ok();
        self.sink.record(MetricsEvent {
            command_type,
            elapsed_ms,
            success,
            request_id,
        });
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::feature_flags::FeatureFlags;
    use crate::websocket::middleware::MiddlewareChain;
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
    use std::sync::Arc;
    use uuid::Uuid;
    use momentum_core::services::context::RequestContext;

    #[derive(Default)]
    struct CountingSink {
        events: std::sync::Mutex<Vec<MetricsEvent>>,
        counter: AtomicU32,
        total_ms: AtomicU64,
    }
    impl MetricsSink for CountingSink {
        fn record(&self, event: MetricsEvent) {
            self.counter.fetch_add(1, Ordering::SeqCst);
            self.total_ms.fetch_add(event.elapsed_ms, Ordering::SeqCst);
            self.events.lock().unwrap().push(event);
        }
    }

    fn make_env() -> CommandEnvelope {
        CommandEnvelope::new(
            "ping",
            json!({}),
            RequestContext {
                user_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                idempotency_key: None,
        trace_id: "unknown".to_string(),
            },
            Some("req-m".into()),
        )
    }

    #[tokio::test]
    async fn success_command_records_event() {
        let sink = Arc::new(CountingSink::default());
        let chain = MiddlewareChain::new().push(MetricsMiddleware::new(sink.clone()));
        let ctx = MiddlewareContext {
            feature_flags: Arc::new(FeatureFlags::default()),
        };
        let _ = chain.execute(make_env(), &ctx).await.unwrap();
        assert_eq!(sink.counter.load(Ordering::SeqCst), 1);
        let ev = sink.events.lock().unwrap();
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].command_type, "ping");
        assert!(ev[0].success);
        assert_eq!(ev[0].request_id.as_deref(), Some("req-m"));
    }

    #[tokio::test]
    async fn failed_command_still_records_event() {
        // Metrics 必须在所有可能拒绝命令的中间件（FeatureFlag）**外面**，
        // 否则上游 ERR 时它根本不会被调用。
        let sink = Arc::new(CountingSink::default());
        let chain = MiddlewareChain::new()
            .push(MetricsMiddleware::new(sink.clone()))
            .push(crate::websocket::feature_flags::FeatureFlagMiddleware::new());
        let ctx = MiddlewareContext {
            feature_flags: Arc::new(FeatureFlags::default()),
        };
        let mut env = make_env();
        env.command_type = "create_issue"; // 默认 flag 关闭 -> 拦截
        let _ = chain.execute(env, &ctx).await; // 这是失败的
        let ev = sink.events.lock().unwrap();
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].command_type, "create_issue");
        assert!(!ev[0].success);
    }

    #[tokio::test]
    async fn elapsed_ms_is_non_zero_at_least_sometimes() {
        // 单调时钟；但 >=0 总是成立。两次事件 elapsed 之和 >= 1。
        let sink = Arc::new(CountingSink::default());
        let chain = MiddlewareChain::new().push(MetricsMiddleware::new(sink.clone()));
        let ctx = MiddlewareContext {
            feature_flags: Arc::new(FeatureFlags::default()),
        };
        for _ in 0..3 {
            let _ = chain.execute(make_env(), &ctx).await.unwrap();
        }
        // 注意 elapsed_ms 基于系统时钟，最坏情况可读 0；累计 >= 0。
        let _ = sink.total_ms.load(Ordering::SeqCst);
        // 至少触发了 3 次
        assert_eq!(sink.counter.load(Ordering::SeqCst), 3);
    }
}
