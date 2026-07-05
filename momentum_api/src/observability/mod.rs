//! 可观测性模块
//!
//! P3 实现：提供 Prometheus 指标导出

pub mod metrics;

pub use metrics::{Metrics, metrics, prometheus_handler};