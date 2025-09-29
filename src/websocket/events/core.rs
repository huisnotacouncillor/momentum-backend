//! 事件系统核心模块
//!
//! 提供事件分发、上下文管理和错误处理的核心功能

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{Event, EventType};
use crate::db::DbPool;
use crate::websocket::auth::AuthenticatedUser;

/// 事件处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResult {
    /// 是否成功
    pub success: bool,
    /// 结果数据
    pub data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 处理器名称
    pub handler_name: String,
    /// 事件ID
    pub event_id: String,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

impl EventResult {
    /// 创建成功结果
    pub fn success(
        data: Option<serde_json::Value>,
        handler_name: String,
        event_id: String,
        execution_time: Duration,
    ) -> Self {
        Self {
            success: true,
            data,
            error: None,
            execution_time_ms: execution_time.as_millis() as u64,
            handler_name,
            event_id,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// 创建失败结果
    pub fn error(
        error_message: String,
        handler_name: String,
        event_id: String,
        execution_time: Duration,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error_message),
            execution_time_ms: execution_time.as_millis() as u64,
            handler_name,
            event_id,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// 事件处理错误
#[derive(Debug, Clone)]
pub enum EventError {
    /// 验证错误
    ValidationError(String),
    /// 权限错误
    PermissionError(String),
    /// 业务错误
    BusinessError(String),
    /// 系统错误
    SystemError(String),
    /// 序列化错误
    SerializationError(String),
    /// 处理器不存在
    HandlerNotFound(String),
    /// 超时错误
    TimeoutError(Duration),
    /// 资源不足
    ResourceExhausted(String),
    /// 依赖错误
    DependencyError(String),
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            EventError::PermissionError(msg) => write!(f, "Permission error: {}", msg),
            EventError::BusinessError(msg) => write!(f, "Business error: {}", msg),
            EventError::SystemError(msg) => write!(f, "System error: {}", msg),
            EventError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            EventError::HandlerNotFound(msg) => write!(f, "Handler not found: {}", msg),
            EventError::TimeoutError(duration) => write!(f, "Timeout after {:?}", duration),
            EventError::ResourceExhausted(msg) => write!(f, "Resource exhausted: {}", msg),
            EventError::DependencyError(msg) => write!(f, "Dependency error: {}", msg),
        }
    }
}

impl StdError for EventError {}

/// 事件处理上下文
#[derive(Debug, Clone)]
pub struct EventContext {
    /// 请求ID，用于追踪
    pub request_id: String,
    /// 认证用户信息
    pub user: Option<AuthenticatedUser>,
    /// 连接ID
    pub connection_id: Option<String>,
    /// 数据库连接池
    pub db: Option<Arc<DbPool>>,
    /// 工作空间ID
    pub workspace_id: Option<Uuid>,
    /// 会话数据
    pub session_data: HashMap<String, serde_json::Value>,
    /// 事件创建时间
    pub created_at: Instant,
    /// 超时时间
    pub timeout: Option<Duration>,
    /// 自定义属性
    pub attributes: HashMap<String, serde_json::Value>,
}

impl EventContext {
    /// 创建新的事件上下文
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            user: None,
            connection_id: None,
            db: None,
            workspace_id: None,
            session_data: HashMap::new(),
            created_at: Instant::now(),
            timeout: Some(Duration::from_secs(30)),
            attributes: HashMap::new(),
        }
    }

    /// 设置用户信息
    pub fn with_user(mut self, user: AuthenticatedUser) -> Self {
        self.workspace_id = user.current_workspace_id;
        self.user = Some(user);
        self
    }

    /// 设置连接ID
    pub fn with_connection_id(mut self, connection_id: String) -> Self {
        self.connection_id = Some(connection_id);
        self
    }

    /// 设置数据库连接
    pub fn with_db(mut self, db: Arc<DbPool>) -> Self {
        self.db = Some(db);
        self
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// 添加属性
    pub fn with_attribute(mut self, key: String, value: serde_json::Value) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// 获取执行时间
    pub fn execution_time(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// 检查是否超时
    pub fn is_timeout(&self) -> bool {
        if let Some(timeout) = self.timeout {
            self.execution_time() > timeout
        } else {
            false
        }
    }

    /// 获取用户ID
    pub fn user_id(&self) -> Option<Uuid> {
        self.user.as_ref().map(|u| u.user_id)
    }

    /// 检查用户权限
    pub fn check_permission(&self, _permission: &str) -> bool {
        // TODO: 实现权限检查逻辑
        // 这里应该根据用户角色和权限系统来检查
        if let Some(_user) = &self.user {
            // 暂时返回true，实际应该检查用户权限
            true
        } else {
            false
        }
    }
}

impl Default for EventContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 事件分发器统计信息
#[derive(Debug, Default, Clone)]
pub struct EventDispatcherStats {
    /// 处理的事件总数
    pub total_events: u64,
    /// 成功处理的事件数
    pub successful_events: u64,
    /// 失败的事件数
    pub failed_events: u64,
    /// 平均处理时间（毫秒）
    pub average_processing_time_ms: f64,
    /// 按类型统计的事件数
    pub events_by_type: HashMap<EventType, u64>,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// 简化的事件分发器
pub struct EventDispatcher {
    /// 统计信息
    stats: Arc<RwLock<EventDispatcherStats>>,
}

impl EventDispatcher {
    /// 创建新的事件分发器
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(EventDispatcherStats::default())),
        }
    }

    /// 分发事件
    pub async fn dispatch(&self, event: impl Event) -> Result<EventResult, EventError> {
        let start_time = Instant::now();
        let event_type = event.event_type();
        let event_id = event.event_id();

        debug!("Dispatching event: {} (type: {:?})", event_id, event_type);

        // 事件验证
        if let Err(e) = event.validate() {
            warn!("Event validation failed: {}", e);
            self.update_stats(event_type, false, start_time.elapsed())
                .await;
            return Err(e);
        }

        // 简单的成功响应
        let result = EventResult::success(
            Some(serde_json::json!({
                "processed": true,
                "event_type": format!("{:?}", event_type),
                "event_id": event_id.clone()
            })),
            "dispatcher".to_string(),
            event_id.clone(),
            start_time.elapsed(),
        );

        self.update_stats(event_type, true, start_time.elapsed())
            .await;

        info!(
            "Event {} processed successfully in {:?}",
            event_id,
            start_time.elapsed()
        );
        Ok(result)
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> EventDispatcherStats {
        (*self.stats.read().await).clone()
    }

    /// 更新统计信息
    async fn update_stats(&self, event_type: EventType, success: bool, duration: Duration) {
        let mut stats = self.stats.write().await;

        stats.total_events += 1;
        if success {
            stats.successful_events += 1;
        } else {
            stats.failed_events += 1;
        }

        // 更新平均处理时间
        let current_avg = stats.average_processing_time_ms;
        let current_duration_ms = duration.as_millis() as f64;
        stats.average_processing_time_ms = (current_avg * (stats.total_events - 1) as f64
            + current_duration_ms)
            / stats.total_events as f64;

        // 更新按类型统计
        *stats.events_by_type.entry(event_type).or_insert(0) += 1;

        stats.last_updated = chrono::Utc::now();
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestEvent {
        id: String,
        data: String,
    }

    impl Event for TestEvent {
        fn event_type(&self) -> EventType {
            EventType::Business
        }

        fn serialize(&self) -> Result<serde_json::Value, EventError> {
            Ok(serde_json::json!({
                "id": self.id,
                "data": self.data
            }))
        }

        fn event_id(&self) -> String {
            self.id.clone()
        }
    }

    #[tokio::test]
    async fn test_event_dispatcher() {
        let dispatcher = EventDispatcher::new();

        let event = TestEvent {
            id: "test-event-1".to_string(),
            data: "test data".to_string(),
        };

        let result = dispatcher.dispatch(event).await.unwrap();
        assert!(result.success);
        assert_eq!(result.handler_name, "dispatcher");
    }

    #[tokio::test]
    async fn test_event_context() {
        let context = EventContext::new()
            .with_timeout(Duration::from_secs(10))
            .with_attribute("test".to_string(), serde_json::json!("value"));

        assert!(context.timeout == Some(Duration::from_secs(10)));
        assert_eq!(
            context.attributes.get("test"),
            Some(&serde_json::json!("value"))
        );
        assert!(!context.is_timeout());
    }

    #[tokio::test]
    async fn test_dispatcher_stats() {
        let dispatcher = EventDispatcher::new();

        let event1 = TestEvent {
            id: "test-1".to_string(),
            data: "data1".to_string(),
        };

        let event2 = TestEvent {
            id: "test-2".to_string(),
            data: "data2".to_string(),
        };

        let _result1 = dispatcher.dispatch(event1).await.unwrap();
        let _result2 = dispatcher.dispatch(event2).await.unwrap();

        let stats = dispatcher.get_stats().await;
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.successful_events, 2);
        assert_eq!(stats.failed_events, 0);
        assert!(stats.average_processing_time_ms >= 0.0);
    }
}
