//! Prometheus 指标导出
//!
//! P3.2 实现：轻量级指标收集器，避免引入 prometheus crate
//!
//! 支持的指标类型：
//! - Counter: 单调递增计数器
//! - Gauge: 瞬时值
//! - Histogram: 直方图（用于延迟）
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use crate::observability::metrics::METRICS;
//!
//! METRICS.http_requests_total
//!     .with_label_values(&["GET", "/users", "200"])
//!     .inc();
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

/// 单个指标值（带标签）
#[derive(Default)]
struct MetricValue {
    count: AtomicU64,
    sum: AtomicU64,    // 用于 Histogram 的总和
    buckets: RwLock<HashMap<u64, AtomicU64>>,  // Histogram bucket 边界值 -> count
}

/// 标签化的指标集合
#[derive(Default)]
pub struct LabeledMetric {
    metrics: RwLock<HashMap<Vec<String>, Arc<MetricValue>>>,
}

impl LabeledMetric {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取或创建指定标签的指标值
    async fn get_or_create(&self, labels: &[&str]) -> Arc<MetricValue> {
        let key: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
        let mut metrics = self.metrics.write().await;
        metrics
            .entry(key)
            .or_insert_with(|| Arc::new(MetricValue::default()))
            .clone()
    }

    /// 增加计数器
    pub async fn inc(&self, labels: &[&str]) {
        let value = self.get_or_create(labels).await;
        value.count.fetch_add(1, Ordering::Relaxed);
    }

    /// 增加指定数量
    pub async fn add(&self, labels: &[&str], amount: u64) {
        let value = self.get_or_create(labels).await;
        value.count.fetch_add(amount, Ordering::Relaxed);
    }

    /// 减少计数器（Gauge 下调）
    pub async fn dec(&self, labels: &[&str]) {
        let value = self.get_or_create(labels).await;
        value.count.fetch_sub(1, Ordering::Relaxed);
    }

    /// 设置瞬时值（Gauge）
    pub async fn set(&self, labels: &[&str], val: u64) {
        let value = self.get_or_create(labels).await;
        value.count.store(val, Ordering::Relaxed);
    }

    /// 记录直方图观测值
    pub async fn observe(&self, labels: &[&str], val: u64, buckets: &[u64]) {
        let value = self.get_or_create(labels).await;
        value.sum.fetch_add(val, Ordering::Relaxed);
        value.count.fetch_add(1, Ordering::Relaxed);

        let mut buckets_map = value.buckets.write().await;
        for &boundary in buckets {
            if val <= boundary {
                buckets_map
                    .entry(boundary)
                    .or_insert_with(|| AtomicU64::new(0))
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// 导出 Prometheus 文本格式
    pub async fn export(&self, name: &str, help: &str, metric_type: &str) -> String {
        let metrics = self.metrics.read().await;
        let mut output = format!("# HELP {} {}\n# TYPE {} {}\n", name, help, name, metric_type);

        for (labels, value) in metrics.iter() {
            let label_str = if labels.is_empty() {
                String::new()
            } else {
                let pairs: Vec<String> = labels
                    .iter()
                    .enumerate()
                    .map(|(i, l)| format!("label{}={:?}", i, l))
                    .collect();
                format!("{{{}}}", pairs.join(","))
            };

            let count = value.count.load(Ordering::Relaxed);
            output.push_str(&format!("{}{} {}\n", name, label_str, count));
        }

        output
    }
}

/// 全局指标集合
pub struct Metrics {
    /// HTTP 请求总数（按 method + path + status）
    pub http_requests_total: LabeledMetric,
    /// HTTP 请求延迟（毫秒）
    pub http_request_duration_ms: LabeledMetric,
    /// 数据库查询总数
    pub db_queries_total: LabeledMetric,
    /// 数据库查询延迟
    pub db_query_duration_ms: LabeledMetric,
    /// WebSocket 连接数（按状态）
    pub ws_connections: LabeledMetric,
    /// WebSocket 消息总数（按方向 + 类型）
    pub ws_messages_total: LabeledMetric,
    /// 错误总数（按类型）
    pub errors_total: LabeledMetric,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            http_requests_total: LabeledMetric::new(),
            http_request_duration_ms: LabeledMetric::new(),
            db_queries_total: LabeledMetric::new(),
            db_query_duration_ms: LabeledMetric::new(),
            ws_connections: LabeledMetric::new(),
            ws_messages_total: LabeledMetric::new(),
            errors_total: LabeledMetric::new(),
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局指标实例（lazy_static 替代方案）
use std::sync::OnceLock;
static METRICS_INSTANCE: OnceLock<Arc<Metrics>> = OnceLock::new();

pub fn metrics() -> &'static Arc<Metrics> {
    METRICS_INSTANCE.get_or_init(|| Arc::new(Metrics::new()))
}

/// Prometheus HTTP handler
pub async fn prometheus_handler() -> impl axum::response::IntoResponse {
    use axum::http::header;

    let m = metrics();
    let mut output = String::new();

    output.push_str(
        &m.http_requests_total
            .export(
                "http_requests_total",
                "Total number of HTTP requests",
                "counter",
            )
            .await,
    );
    output.push_str(
        &m.http_request_duration_ms
            .export(
                "http_request_duration_ms",
                "HTTP request duration in milliseconds",
                "histogram",
            )
            .await,
    );
    output.push_str(
        &m.db_queries_total
            .export(
                "db_queries_total",
                "Total number of database queries",
                "counter",
            )
            .await,
    );
    output.push_str(
        &m.db_query_duration_ms
            .export(
                "db_query_duration_ms",
                "Database query duration in milliseconds",
                "histogram",
            )
            .await,
    );
    output.push_str(
        &m.ws_connections
            .export(
                "ws_connections",
                "Number of active WebSocket connections by state",
                "gauge",
            )
            .await,
    );
    output.push_str(
        &m.ws_messages_total
            .export(
                "ws_messages_total",
                "Total WebSocket messages by direction and type",
                "counter",
            )
            .await,
    );
    output.push_str(
        &m.errors_total
            .export("errors_total", "Total errors by type", "counter")
            .await,
    );

    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        output,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_counter_inc() {
        let m = LabeledMetric::new();
        m.inc(&["GET", "/users", "200"]).await;
        m.inc(&["GET", "/users", "200"]).await;
        m.inc(&["POST", "/users", "201"]).await;

        let output = m.export("test", "Test metric", "counter").await;
        assert!(output.contains("test{label0=\"GET\""));
        assert!(output.contains("test{label0=\"POST\""));
    }

    #[tokio::test]
    async fn test_gauge_set() {
        let m = LabeledMetric::new();
        m.set(&["active"], 42).await;
        m.set(&["active"], 100).await;

        let output = m.export("gauge_test", "Gauge", "gauge").await;
        assert!(output.contains("gauge_test{label0=\"active\"} 100"));
    }

    #[tokio::test]
    async fn test_histogram_observe() {
        let m = LabeledMetric::new();
        m.observe(&["api"], 50, &[10, 100, 1000]).await;
        m.observe(&["api"], 150, &[10, 100, 1000]).await;
        m.observe(&["api"], 500, &[10, 100, 1000]).await;

        let output = m.export("hist_test", "Histogram", "histogram").await;
        // 100ms 桶：50 + 150 = 2
        // 1000ms 桶：50 + 150 + 500 = 3
        assert!(output.contains("hist_test{label0=\"api\"} 3"));
    }

    #[tokio::test]
    async fn test_prometheus_handler() {
        use axum::response::IntoResponse;

        // 调用 handler
        let response = prometheus_handler().await.into_response();
        assert_eq!(response.status(), 200);
    }
}