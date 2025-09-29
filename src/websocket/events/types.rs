//! 事件系统类型定义
//!
//! 定义事件系统中使用的核心类型、枚举和常量

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// 事件类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// 连接事件
    Connection,
    /// 消息事件
    Message,
    /// 业务命令事件
    Command,
    /// 系统事件
    System,
    /// 业务事件
    Business,
    /// 错误事件
    Error,
    /// 通知事件
    Notification,
    /// 监控事件
    Monitoring,
    /// 自定义事件
    Custom,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::Connection => write!(f, "connection"),
            EventType::Message => write!(f, "message"),
            EventType::Command => write!(f, "command"),
            EventType::System => write!(f, "system"),
            EventType::Business => write!(f, "business"),
            EventType::Error => write!(f, "error"),
            EventType::Notification => write!(f, "notification"),
            EventType::Monitoring => write!(f, "monitoring"),
            EventType::Custom => write!(f, "custom"),
        }
    }
}

/// 事件优先级
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPriority {
    /// 低优先级
    Low,
    /// 普通优先级
    Normal,
    /// 高优先级
    High,
    /// 紧急优先级
    Critical,
}

impl Default for EventPriority {
    fn default() -> Self {
        EventPriority::Normal
    }
}

impl fmt::Display for EventPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventPriority::Low => write!(f, "low"),
            EventPriority::Normal => write!(f, "normal"),
            EventPriority::High => write!(f, "high"),
            EventPriority::Critical => write!(f, "critical"),
        }
    }
}

impl From<u32> for EventPriority {
    fn from(value: u32) -> Self {
        match value {
            0..=25 => EventPriority::Critical,
            26..=50 => EventPriority::High,
            51..=100 => EventPriority::Normal,
            _ => EventPriority::Low,
        }
    }
}

impl From<EventPriority> for u32 {
    fn from(priority: EventPriority) -> Self {
        match priority {
            EventPriority::Critical => 10,
            EventPriority::High => 25,
            EventPriority::Normal => 50,
            EventPriority::Low => 100,
        }
    }
}

/// 事件元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventMetadata {
    /// 事件版本
    pub version: String,
    /// 事件来源
    pub source: Option<String>,
    /// 相关联的事件ID
    pub correlation_id: Option<String>,
    /// 因果关系ID（用于事件链跟踪）
    pub causation_id: Option<String>,
    /// 事件标签
    pub labels: HashMap<String, String>,
    /// 自定义属性
    pub attributes: HashMap<String, serde_json::Value>,
    /// 事件过期时间
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 是否为重试事件
    pub is_retry: bool,
}

impl EventMetadata {
    /// 创建新的元数据
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置版本
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// 设置来源
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// 设置关联ID
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// 设置因果关系ID
    pub fn with_causation_id(mut self, id: impl Into<String>) -> Self {
        self.causation_id = Some(id.into());
        self
    }

    /// 添加标签
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// 添加属性
    pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// 设置过期时间
    pub fn with_expiration(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// 设置重试配置
    pub fn with_retry_config(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// 增加重试次数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.is_retry = true;
    }
}

/// 事件状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    /// 待处理
    Pending,
    /// 处理中
    Processing,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 已过期
    Expired,
    /// 重试中
    Retrying,
}

impl Default for EventStatus {
    fn default() -> Self {
        EventStatus::Pending
    }
}

impl fmt::Display for EventStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventStatus::Pending => write!(f, "pending"),
            EventStatus::Processing => write!(f, "processing"),
            EventStatus::Completed => write!(f, "completed"),
            EventStatus::Failed => write!(f, "failed"),
            EventStatus::Cancelled => write!(f, "cancelled"),
            EventStatus::Expired => write!(f, "expired"),
            EventStatus::Retrying => write!(f, "retrying"),
        }
    }
}

/// 事件处理模式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventProcessingMode {
    /// 同步处理
    Sync,
    /// 异步处理
    Async,
    /// 批量处理
    Batch,
    /// 流处理
    Stream,
    /// 延迟处理
    Delayed,
}

impl Default for EventProcessingMode {
    fn default() -> Self {
        EventProcessingMode::Async
    }
}

/// 事件传播模式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPropagationMode {
    /// 单播 - 发送给单个处理器
    Unicast,
    /// 多播 - 发送给多个指定处理器
    Multicast,
    /// 广播 - 发送给所有处理器
    Broadcast,
    /// 发布订阅 - 基于主题的发送
    PubSub,
}

impl Default for EventPropagationMode {
    fn default() -> Self {
        EventPropagationMode::Unicast
    }
}

/// 事件序列化格式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSerializationFormat {
    /// JSON格式
    Json,
    /// MessagePack格式
    MessagePack,
    /// Protocol Buffers格式
    Protobuf,
    /// 二进制格式
    Binary,
    /// 自定义格式
    Custom(String),
}

impl Default for EventSerializationFormat {
    fn default() -> Self {
        EventSerializationFormat::Json
    }
}

/// 事件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    /// 事件类型
    pub event_type: EventType,
    /// 处理模式
    pub processing_mode: EventProcessingMode,
    /// 传播模式
    pub propagation_mode: EventPropagationMode,
    /// 序列化格式
    pub serialization_format: EventSerializationFormat,
    /// 默认优先级
    pub default_priority: EventPriority,
    /// 超时时间（秒）
    pub timeout_seconds: u64,
    /// 是否启用重试
    pub retry_enabled: bool,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（秒）
    pub retry_interval_seconds: u64,
    /// 是否启用缓存
    pub cache_enabled: bool,
    /// 缓存TTL（秒）
    pub cache_ttl_seconds: u64,
    /// 是否启用持久化
    pub persistence_enabled: bool,
    /// 批处理大小
    pub batch_size: u32,
    /// 批处理超时（毫秒）
    pub batch_timeout_ms: u64,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            event_type: EventType::Business,
            processing_mode: EventProcessingMode::Async,
            propagation_mode: EventPropagationMode::Unicast,
            serialization_format: EventSerializationFormat::Json,
            default_priority: EventPriority::Normal,
            timeout_seconds: 30,
            retry_enabled: false,
            max_retries: 3,
            retry_interval_seconds: 1,
            cache_enabled: false,
            cache_ttl_seconds: 300,
            persistence_enabled: false,
            batch_size: 10,
            batch_timeout_ms: 1000,
        }
    }
}

impl EventConfig {
    /// 创建连接事件配置
    pub fn connection_config() -> Self {
        Self {
            event_type: EventType::Connection,
            processing_mode: EventProcessingMode::Sync,
            propagation_mode: EventPropagationMode::Broadcast,
            default_priority: EventPriority::High,
            timeout_seconds: 5,
            ..Default::default()
        }
    }

    /// 创建消息事件配置
    pub fn message_config() -> Self {
        Self {
            event_type: EventType::Message,
            processing_mode: EventProcessingMode::Async,
            propagation_mode: EventPropagationMode::Multicast,
            cache_enabled: true,
            persistence_enabled: true,
            ..Default::default()
        }
    }

    /// 创建业务事件配置
    pub fn business_config() -> Self {
        Self {
            event_type: EventType::Business,
            processing_mode: EventProcessingMode::Async,
            retry_enabled: true,
            persistence_enabled: true,
            batch_size: 50,
            ..Default::default()
        }
    }

    /// 创建系统事件配置
    pub fn system_config() -> Self {
        Self {
            event_type: EventType::System,
            processing_mode: EventProcessingMode::Sync,
            propagation_mode: EventPropagationMode::Broadcast,
            default_priority: EventPriority::Critical,
            timeout_seconds: 10,
            ..Default::default()
        }
    }

    /// 创建监控事件配置
    pub fn monitoring_config() -> Self {
        Self {
            event_type: EventType::Monitoring,
            processing_mode: EventProcessingMode::Batch,
            propagation_mode: EventPropagationMode::PubSub,
            default_priority: EventPriority::Low,
            batch_size: 100,
            batch_timeout_ms: 5000,
            ..Default::default()
        }
    }
}

/// 事件源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSource {
    /// 源标识符
    pub id: String,
    /// 源名称
    pub name: String,
    /// 源类型
    pub source_type: EventSourceType,
    /// 源版本
    pub version: String,
    /// 源描述
    pub description: Option<String>,
    /// 源元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 事件源类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSourceType {
    /// WebSocket连接
    WebSocket,
    /// HTTP API
    HttpApi,
    /// 数据库触发器
    DatabaseTrigger,
    /// 消息队列
    MessageQueue,
    /// 定时任务
    ScheduledTask,
    /// 外部系统
    ExternalSystem,
    /// 内部服务
    InternalService,
}

/// 事件目标信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTarget {
    /// 目标标识符
    pub id: String,
    /// 目标类型
    pub target_type: EventTargetType,
    /// 目标地址
    pub address: String,
    /// 过滤条件
    pub filters: HashMap<String, serde_json::Value>,
    /// 是否激活
    pub active: bool,
}

/// 事件目标类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventTargetType {
    /// WebSocket连接
    WebSocket,
    /// HTTP回调
    HttpCallback,
    /// 消息队列
    MessageQueue,
    /// 数据库
    Database,
    /// 文件系统
    FileSystem,
    /// 外部API
    ExternalApi,
}

/// 事件路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRoutingRule {
    /// 规则ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 匹配条件
    pub conditions: Vec<EventCondition>,
    /// 目标列表
    pub targets: Vec<EventTarget>,
    /// 规则优先级
    pub priority: u32,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 事件条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCondition {
    /// 字段名称
    pub field: String,
    /// 操作符
    pub operator: EventOperator,
    /// 比较值
    pub value: serde_json::Value,
    /// 逻辑关系
    pub logic: EventLogic,
}

/// 事件操作符
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventOperator {
    /// 等于
    Equals,
    /// 不等于
    NotEquals,
    /// 大于
    GreaterThan,
    /// 大于等于
    GreaterThanOrEqual,
    /// 小于
    LessThan,
    /// 小于等于
    LessThanOrEqual,
    /// 包含
    Contains,
    /// 不包含
    NotContains,
    /// 开始于
    StartsWith,
    /// 结束于
    EndsWith,
    /// 正则匹配
    Regex,
    /// 存在
    Exists,
    /// 不存在
    NotExists,
    /// 在列表中
    In,
    /// 不在列表中
    NotIn,
}

/// 事件逻辑关系
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventLogic {
    /// 与
    And,
    /// 或
    Or,
    /// 非
    Not,
}

/// 事件指标
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventMetrics {
    /// 事件总数
    pub total_events: u64,
    /// 成功处理数
    pub successful_events: u64,
    /// 失败处理数
    pub failed_events: u64,
    /// 平均处理时间（毫秒）
    pub average_processing_time_ms: f64,
    /// 最大处理时间（毫秒）
    pub max_processing_time_ms: u64,
    /// 最小处理时间（毫秒）
    pub min_processing_time_ms: u64,
    /// 按类型统计
    pub events_by_type: HashMap<EventType, u64>,
    /// 按状态统计
    pub events_by_status: HashMap<EventStatus, u64>,
    /// 按优先级统计
    pub events_by_priority: HashMap<EventPriority, u64>,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl EventMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新成功事件统计
    pub fn record_success(&mut self, event_type: EventType, processing_time_ms: u64) {
        self.total_events += 1;
        self.successful_events += 1;
        *self.events_by_type.entry(event_type).or_insert(0) += 1;
        *self
            .events_by_status
            .entry(EventStatus::Completed)
            .or_insert(0) += 1;

        self.update_processing_time(processing_time_ms);
        self.last_updated = chrono::Utc::now();
    }

    /// 更新失败事件统计
    pub fn record_failure(&mut self, event_type: EventType, processing_time_ms: u64) {
        self.total_events += 1;
        self.failed_events += 1;
        *self.events_by_type.entry(event_type).or_insert(0) += 1;
        *self
            .events_by_status
            .entry(EventStatus::Failed)
            .or_insert(0) += 1;

        self.update_processing_time(processing_time_ms);
        self.last_updated = chrono::Utc::now();
    }

    /// 更新处理时间统计
    fn update_processing_time(&mut self, processing_time_ms: u64) {
        if self.total_events == 1 {
            self.average_processing_time_ms = processing_time_ms as f64;
            self.max_processing_time_ms = processing_time_ms;
            self.min_processing_time_ms = processing_time_ms;
        } else {
            // 更新平均处理时间
            self.average_processing_time_ms = (self.average_processing_time_ms
                * (self.total_events - 1) as f64
                + processing_time_ms as f64)
                / self.total_events as f64;

            // 更新最大最小值
            self.max_processing_time_ms = self.max_processing_time_ms.max(processing_time_ms);
            self.min_processing_time_ms = self.min_processing_time_ms.min(processing_time_ms);
        }
    }

    /// 计算成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_events == 0 {
            0.0
        } else {
            self.successful_events as f64 / self.total_events as f64 * 100.0
        }
    }

    /// 计算失败率
    pub fn failure_rate(&self) -> f64 {
        if self.total_events == 0 {
            0.0
        } else {
            self.failed_events as f64 / self.total_events as f64 * 100.0
        }
    }
}

/// 常量定义
pub mod constants {
    /// 默认事件超时时间（秒）
    pub const DEFAULT_EVENT_TIMEOUT_SECONDS: u64 = 30;

    /// 默认批处理大小
    pub const DEFAULT_BATCH_SIZE: u32 = 10;

    /// 默认批处理超时（毫秒）
    pub const DEFAULT_BATCH_TIMEOUT_MS: u64 = 1000;

    /// 默认重试次数
    pub const DEFAULT_MAX_RETRIES: u32 = 3;

    /// 默认重试间隔（秒）
    pub const DEFAULT_RETRY_INTERVAL_SECONDS: u64 = 1;

    /// 默认缓存TTL（秒）
    pub const DEFAULT_CACHE_TTL_SECONDS: u64 = 300;

    /// 最大事件大小（字节）
    pub const MAX_EVENT_SIZE_BYTES: usize = 1024 * 1024; // 1MB

    /// 最大元数据大小（字节）
    pub const MAX_METADATA_SIZE_BYTES: usize = 64 * 1024; // 64KB

    /// 事件ID长度
    pub const EVENT_ID_LENGTH: usize = 36; // UUID长度

    /// 最大标签数量
    pub const MAX_LABELS_COUNT: usize = 50;

    /// 最大属性数量
    pub const MAX_ATTRIBUTES_COUNT: usize = 100;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_priority_conversion() {
        assert_eq!(EventPriority::from(5u32), EventPriority::Critical);
        assert_eq!(EventPriority::from(30u32), EventPriority::High);
        assert_eq!(EventPriority::from(75u32), EventPriority::Normal);
        assert_eq!(EventPriority::from(200u32), EventPriority::Low);

        assert_eq!(u32::from(EventPriority::Critical), 10);
        assert_eq!(u32::from(EventPriority::High), 25);
        assert_eq!(u32::from(EventPriority::Normal), 50);
        assert_eq!(u32::from(EventPriority::Low), 100);
    }

    #[test]
    fn test_event_metadata() {
        let metadata = EventMetadata::new()
            .with_version("1.0")
            .with_source("test_service")
            .with_label("environment", "development")
            .with_attribute("custom_field", serde_json::json!("custom_value"));

        assert_eq!(metadata.version, "1.0");
        assert_eq!(metadata.source, Some("test_service".to_string()));
        assert_eq!(
            metadata.labels.get("environment"),
            Some(&"development".to_string())
        );
        assert!(metadata.attributes.contains_key("custom_field"));
    }

    #[test]
    fn test_event_config() {
        let config = EventConfig::business_config();
        assert_eq!(config.event_type, EventType::Business);
        assert!(config.retry_enabled);
        assert!(config.persistence_enabled);
        assert_eq!(config.batch_size, 50);
    }

    #[test]
    fn test_event_metrics() {
        let mut metrics = EventMetrics::new();

        metrics.record_success(EventType::Business, 100);
        metrics.record_failure(EventType::Business, 200);

        assert_eq!(metrics.total_events, 2);
        assert_eq!(metrics.successful_events, 1);
        assert_eq!(metrics.failed_events, 1);
        assert_eq!(metrics.success_rate(), 50.0);
        assert_eq!(metrics.failure_rate(), 50.0);
        assert_eq!(metrics.average_processing_time_ms, 150.0);
    }

    #[test]
    fn test_event_metadata_expiration() {
        let mut metadata = EventMetadata::new();
        assert!(!metadata.is_expired());

        let past_time = chrono::Utc::now() - chrono::Duration::hours(1);
        metadata = metadata.with_expiration(past_time);
        assert!(metadata.is_expired());

        let future_time = chrono::Utc::now() + chrono::Duration::hours(1);
        metadata = metadata.with_expiration(future_time);
        assert!(!metadata.is_expired());
    }

    #[test]
    fn test_event_metadata_retry() {
        let mut metadata = EventMetadata::new().with_retry_config(3);

        assert!(metadata.can_retry());
        assert_eq!(metadata.retry_count, 0);
        assert!(!metadata.is_retry);

        metadata.increment_retry();
        assert_eq!(metadata.retry_count, 1);
        assert!(metadata.is_retry);
        assert!(metadata.can_retry());

        // 超过最大重试次数
        metadata.retry_count = 3;
        assert!(!metadata.can_retry());
    }
}
