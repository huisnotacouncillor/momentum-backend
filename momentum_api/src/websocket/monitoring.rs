use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_connections: u64,
    pub active_connections: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_commands_processed: u64,
    pub average_response_time_ms: f64,
    pub error_rate: f64,
    pub last_updated: DateTime<Utc>,
}

/// 连接质量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionQuality {
    pub user_id: Uuid,
    pub connection_id: String,
    pub latency_ms: f64,
    pub packet_loss_rate: f64,
    pub last_ping_time: DateTime<Utc>,
    pub connection_stability: f64, // 0.0-1.0, 1.0表示最稳定
    pub bandwidth_usage: u64,      // bytes per second
}

/// 系统健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// 系统健康检查
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub status: HealthStatus,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub details: HashMap<String, serde_json::Value>,
}

/// 监控数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringData {
    pub performance: PerformanceMetrics,
    pub connection_quality: Vec<ConnectionQuality>,
    pub health_checks: Vec<HealthCheck>,
    pub error_summary: HashMap<String, u64>,
    pub resource_usage: ResourceUsage,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_io_mbps: f64,
}

/// WebSocket监控器
#[derive(Clone)]
pub struct WebSocketMonitor {
    /// 性能指标
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// 连接质量数据
    connection_quality: Arc<RwLock<HashMap<String, ConnectionQuality>>>,
    /// 健康检查结果
    health_checks: Arc<RwLock<Vec<HealthCheck>>>,
    /// 错误摘要
    error_summary: Arc<RwLock<HashMap<String, u64>>>,
    /// 响应时间记录
    response_times: Arc<RwLock<Vec<Duration>>>,
    /// 监控配置
    config: MonitoringConfig,
}

/// 监控配置
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub max_response_time_samples: usize,
    pub health_check_interval: Duration,
    pub metrics_collection_interval: Duration,
    pub connection_quality_threshold_ms: f64,
    pub error_rate_threshold: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            max_response_time_samples: 1000,
            health_check_interval: Duration::from_secs(30),
            metrics_collection_interval: Duration::from_secs(10),
            connection_quality_threshold_ms: 100.0,
            error_rate_threshold: 0.05, // 5%
        }
    }
}

impl WebSocketMonitor {
    pub fn new(config: MonitoringConfig) -> Self {
        let monitor = Self {
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                total_connections: 0,
                active_connections: 0,
                total_messages_sent: 0,
                total_messages_received: 0,
                total_commands_processed: 0,
                average_response_time_ms: 0.0,
                error_rate: 0.0,
                last_updated: Utc::now(),
            })),
            connection_quality: Arc::new(RwLock::new(HashMap::new())),
            health_checks: Arc::new(RwLock::new(Vec::new())),
            error_summary: Arc::new(RwLock::new(HashMap::new())),
            response_times: Arc::new(RwLock::new(Vec::new())),
            config,
        };

        // 启动后台监控任务
        monitor.start_background_tasks();
        monitor
    }

    /// 记录新连接
    pub async fn record_connection(&self, user_id: Uuid, connection_id: String) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.total_connections += 1;
        metrics.active_connections += 1;
        metrics.last_updated = Utc::now();

        // 初始化连接质量数据
        let mut quality_map = self.connection_quality.write().unwrap();
        let connection_id_clone = connection_id.clone();
        quality_map.insert(
            connection_id_clone.clone(),
            ConnectionQuality {
                user_id,
                connection_id: connection_id_clone.clone(),
                latency_ms: 0.0,
                packet_loss_rate: 0.0,
                last_ping_time: Utc::now(),
                connection_stability: 1.0,
                bandwidth_usage: 0,
            },
        );

        info!(
            "New connection recorded: user_id={}, connection_id={}",
            user_id, connection_id_clone
        );
    }

    /// 记录连接断开
    pub async fn record_disconnection(&self, connection_id: &str) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.active_connections = metrics.active_connections.saturating_sub(1);
        metrics.last_updated = Utc::now();

        let mut quality_map = self.connection_quality.write().unwrap();
        quality_map.remove(connection_id);

        debug!(
            "Connection disconnection recorded: connection_id={}",
            connection_id
        );
    }

    /// 记录消息发送
    pub async fn record_message_sent(&self, connection_id: &str, message_size: usize) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.total_messages_sent += 1;
        metrics.last_updated = Utc::now();

        // 更新带宽使用
        let mut quality_map = self.connection_quality.write().unwrap();
        if let Some(quality) = quality_map.get_mut(connection_id) {
            quality.bandwidth_usage += message_size as u64;
        }

        debug!(
            "Message sent recorded: connection_id={}, size={}",
            connection_id, message_size
        );
    }

    /// 记录消息接收
    pub async fn record_message_received(&self, connection_id: &str, message_size: usize) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.total_messages_received += 1;
        metrics.last_updated = Utc::now();

        // 更新带宽使用
        let mut quality_map = self.connection_quality.write().unwrap();
        if let Some(quality) = quality_map.get_mut(connection_id) {
            quality.bandwidth_usage += message_size as u64;
        }

        debug!(
            "Message received recorded: connection_id={}, size={}",
            connection_id, message_size
        );
    }

    /// 记录命令处理
    pub async fn record_command_processed(&self, response_time: Duration, success: bool) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.total_commands_processed += 1;
        metrics.last_updated = Utc::now();

        // 记录响应时间
        let mut response_times = self.response_times.write().unwrap();
        response_times.push(response_time);
        if response_times.len() > self.config.max_response_time_samples {
            response_times.remove(0);
        }

        // 计算平均响应时间
        let total_ms: f64 = response_times.iter().map(|d| d.as_millis() as f64).sum();
        metrics.average_response_time_ms = total_ms / response_times.len() as f64;

        // 更新错误率
        if !success {
            let mut error_summary = self.error_summary.write().unwrap();
            *error_summary
                .entry("command_failed".to_string())
                .or_insert(0) += 1;
        }

        // 计算错误率
        let total_commands = metrics.total_commands_processed;
        let total_errors: u64 = self.error_summary.read().unwrap().values().sum();
        metrics.error_rate = if total_commands > 0 {
            total_errors as f64 / total_commands as f64
        } else {
            0.0
        };

        debug!(
            "Command processed recorded: response_time={:?}, success={}",
            response_time, success
        );
    }

    /// 记录连接质量数据
    pub async fn record_connection_quality(
        &self,
        connection_id: &str,
        latency_ms: f64,
        packet_loss_rate: f64,
    ) {
        let mut quality_map = self.connection_quality.write().unwrap();
        if let Some(quality) = quality_map.get_mut(connection_id) {
            quality.latency_ms = latency_ms;
            quality.packet_loss_rate = packet_loss_rate;
            quality.last_ping_time = Utc::now();

            // 计算连接稳定性（基于延迟和丢包率）
            let latency_score = if latency_ms <= self.config.connection_quality_threshold_ms {
                1.0
            } else {
                (self.config.connection_quality_threshold_ms / latency_ms).min(1.0)
            };
            let packet_loss_score = (1.0 - packet_loss_rate).max(0.0);
            quality.connection_stability = (latency_score + packet_loss_score) / 2.0;

            debug!(
                "Connection quality updated: connection_id={}, latency={}ms, packet_loss={}%",
                connection_id,
                latency_ms,
                packet_loss_rate * 100.0
            );
        }
    }

    /// 记录错误
    pub async fn record_error(&self, error_type: &str, error_message: &str) {
        let mut error_summary = self.error_summary.write().unwrap();
        *error_summary.entry(error_type.to_string()).or_insert(0) += 1;

        // 记录详细错误信息
        warn!(
            "Error recorded: type={}, message={}",
            error_type, error_message
        );
    }

    /// 获取监控数据
    pub async fn get_monitoring_data(&self) -> MonitoringData {
        let metrics = self.metrics.read().unwrap().clone();
        let connection_quality: Vec<ConnectionQuality> = self
            .connection_quality
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect();
        let health_checks = self.health_checks.read().unwrap().clone();
        let error_summary = self.error_summary.read().unwrap().clone();

        MonitoringData {
            performance: metrics,
            connection_quality,
            health_checks,
            error_summary,
            resource_usage: self.get_resource_usage().await,
        }
    }

    /// 获取资源使用情况
    async fn get_resource_usage(&self) -> ResourceUsage {
        // 这里应该集成实际的系统监控库
        // 暂时返回模拟数据
        ResourceUsage {
            memory_usage_mb: 128.5,
            cpu_usage_percent: 15.2,
            disk_usage_percent: 45.8,
            network_io_mbps: 12.3,
        }
    }

    /// 执行健康检查
    async fn perform_health_check(&self) {
        let mut health_checks = Vec::new();

        // 检查连接数
        let metrics = self.metrics.read().unwrap();
        if metrics.active_connections > 1000 {
            health_checks.push(HealthCheck {
                status: HealthStatus::Warning,
                message: "High number of active connections".to_string(),
                timestamp: Utc::now(),
                details: HashMap::new(),
            });
        }

        // 检查错误率
        if metrics.error_rate > self.config.error_rate_threshold {
            health_checks.push(HealthCheck {
                status: HealthStatus::Critical,
                message: format!("High error rate: {:.2}%", metrics.error_rate * 100.0),
                timestamp: Utc::now(),
                details: HashMap::new(),
            });
        }

        // 检查平均响应时间
        if metrics.average_response_time_ms > 1000.0 {
            health_checks.push(HealthCheck {
                status: HealthStatus::Warning,
                message: format!(
                    "High average response time: {:.2}ms",
                    metrics.average_response_time_ms
                ),
                timestamp: Utc::now(),
                details: HashMap::new(),
            });
        }

        // 检查连接质量
        let quality_map = self.connection_quality.read().unwrap();
        let poor_connections = quality_map
            .values()
            .filter(|q| q.connection_stability < 0.5)
            .count();

        if poor_connections > 0 {
            health_checks.push(HealthCheck {
                status: HealthStatus::Warning,
                message: format!("{} connections with poor quality", poor_connections),
                timestamp: Utc::now(),
                details: HashMap::new(),
            });
        }

        // 更新健康检查结果
        let mut health_checks_storage = self.health_checks.write().unwrap();
        *health_checks_storage = health_checks;

        info!(
            "Health check completed: {} issues found",
            health_checks_storage.len()
        );
    }

    /// 启动后台监控任务
    fn start_background_tasks(&self) {
        let monitor = self.clone();
        let config = self.config.clone();

        // 健康检查任务
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.health_check_interval);
            loop {
                interval.tick().await;
                monitor.perform_health_check().await;
            }
        });

        // 指标收集任务
        let monitor = self.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.metrics_collection_interval);
            loop {
                interval.tick().await;
                monitor.collect_metrics().await;
            }
        });
    }

    /// 收集指标
    async fn collect_metrics(&self) {
        // 清理过期的连接质量数据
        let mut quality_map = self.connection_quality.write().unwrap();
        let cutoff_time = Utc::now() - chrono::Duration::minutes(5);
        quality_map.retain(|_, quality| quality.last_ping_time > cutoff_time);

        debug!("Metrics collection completed");
    }

    /// 获取性能指标
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// 获取连接质量列表
    pub async fn get_connection_quality(&self) -> Vec<ConnectionQuality> {
        self.connection_quality
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// 获取健康检查结果
    pub async fn get_health_checks(&self) -> Vec<HealthCheck> {
        self.health_checks.read().unwrap().clone()
    }

    /// 重置指标
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().unwrap();
        *metrics = PerformanceMetrics {
            total_connections: 0,
            active_connections: 0,
            total_messages_sent: 0,
            total_messages_received: 0,
            total_commands_processed: 0,
            average_response_time_ms: 0.0,
            error_rate: 0.0,
            last_updated: Utc::now(),
        };

        let mut response_times = self.response_times.write().unwrap();
        response_times.clear();

        let mut error_summary = self.error_summary.write().unwrap();
        error_summary.clear();

        info!("Metrics reset completed");
    }
}

impl Default for WebSocketMonitor {
    fn default() -> Self {
        Self::new(MonitoringConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_connection_lifecycle() {
        let monitor = WebSocketMonitor::new(MonitoringConfig::default());
        let user_id = Uuid::new_v4();
        let connection_id = "test-connection".to_string();

        // 记录连接
        monitor
            .record_connection(user_id, connection_id.clone())
            .await;
        let metrics = monitor.get_performance_metrics().await;
        assert_eq!(metrics.total_connections, 1);
        assert_eq!(metrics.active_connections, 1);

        // 记录消息
        monitor.record_message_sent(&connection_id, 100).await;
        monitor.record_message_received(&connection_id, 50).await;
        let metrics = monitor.get_performance_metrics().await;
        assert_eq!(metrics.total_messages_sent, 1);
        assert_eq!(metrics.total_messages_received, 1);

        // 记录连接断开
        monitor.record_disconnection(&connection_id).await;
        let metrics = monitor.get_performance_metrics().await;
        assert_eq!(metrics.active_connections, 0);
    }

    #[tokio::test]
    async fn test_command_processing_metrics() {
        let monitor = WebSocketMonitor::new(MonitoringConfig::default());

        // 记录成功的命令
        monitor
            .record_command_processed(Duration::from_millis(50), true)
            .await;
        monitor
            .record_command_processed(Duration::from_millis(100), true)
            .await;

        // 记录失败的命令
        monitor
            .record_command_processed(Duration::from_millis(200), false)
            .await;

        let metrics = monitor.get_performance_metrics().await;
        assert_eq!(metrics.total_commands_processed, 3);
        assert_eq!(metrics.error_rate, 1.0 / 3.0);
        assert!(metrics.average_response_time_ms > 0.0);
    }

    #[tokio::test]
    async fn test_connection_quality() {
        let monitor = WebSocketMonitor::new(MonitoringConfig::default());
        let user_id = Uuid::new_v4();
        let connection_id = "test-connection".to_string();

        monitor
            .record_connection(user_id, connection_id.clone())
            .await;
        monitor
            .record_connection_quality(&connection_id, 50.0, 0.01)
            .await;

        let quality_list = monitor.get_connection_quality().await;
        assert_eq!(quality_list.len(), 1);
        assert_eq!(quality_list[0].latency_ms, 50.0);
        assert_eq!(quality_list[0].packet_loss_rate, 0.01);
        assert!(quality_list[0].connection_stability > 0.0);
    }
}
