//! 事件处理器注册表和处理器实现
//!
//! 提供简化的事件处理器注册、查找和执行机制

use serde::Serialize;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{
    Event, EventType,
    core::{EventContext, EventError, EventResult},
};

/// 事件处理器特征
pub trait EventHandler<E: Event>: Send + Sync {
    type Response: Send + Sync + Serialize;
    type Error: StdError + Send + Sync;

    /// 处理事件
    fn handle<'a>(
        &'a self,
        event: &'a E,
        ctx: &'a EventContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>,
    >;

    /// 处理器名称
    fn name(&self) -> &'static str;

    /// 是否可以处理该事件
    fn can_handle(&self, event: &E) -> bool {
        true
    }

    /// 处理器优先级
    fn priority(&self) -> u32 {
        100
    }

    /// 处理器描述
    fn description(&self) -> &'static str {
        "Event handler"
    }

    /// 是否支持并行处理
    fn supports_parallel(&self) -> bool {
        true
    }

    /// 最大执行时间
    fn max_execution_time(&self) -> Duration {
        Duration::from_secs(30)
    }
}

/// 处理器统计信息
#[derive(Debug, Default, Clone)]
pub struct HandlerStats {
    pub total_handlers: usize,
    pub execution_stats: HashMap<String, HandlerExecutionStats>,
}

#[derive(Debug, Default, Clone)]
pub struct HandlerExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

/// 处理器注册表
pub struct HandlerRegistry {
    /// 处理器统计信息
    stats: Arc<RwLock<HandlerStats>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(HandlerStats::default())),
        }
    }

    /// 注册事件处理器（简化版本）
    pub async fn register<E: Event + 'static>(&self, _handler_name: &str) {
        let mut stats = self.stats.write().await;
        stats.total_handlers += 1;
        info!("Registered handler: {}", _handler_name);
    }

    /// 查找匹配的处理器（简化版本）
    pub async fn find_handlers(&self, _event: &dyn Event) -> Vec<String> {
        // 简化实现，返回默认处理器名称
        vec!["default_handler".to_string()]
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> HandlerStats {
        (*self.stats.read().await).clone()
    }

    /// 重置统计信息
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = HandlerStats::default();
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 默认的事件处理器实现

/// 连接事件处理器
pub struct ConnectionEventHandler;

impl EventHandler<crate::websocket::events::GenericWebSocketEvent> for ConnectionEventHandler {
    type Response = serde_json::Value;
    type Error = EventError;

    fn handle<'a>(
        &'a self,
        event: &'a crate::websocket::events::GenericWebSocketEvent,
        _ctx: &'a EventContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>,
    > {
        Box::pin(async move {
            use crate::websocket::events::{WebSocketEvent, WebSocketEventType};

            match &event.event_type {
                WebSocketEventType::Connection {
                    action,
                    user_id,
                    connection_id,
                } => {
                    info!(
                        "Processing connection event: {:?} for user {} connection {}",
                        action, user_id, connection_id
                    );
                    Ok(serde_json::json!({
                        "status": "processed",
                        "action": format!("{:?}", action),
                        "user_id": user_id,
                        "connection_id": connection_id,
                        "timestamp": chrono::Utc::now()
                    }))
                }
                _ => Ok(serde_json::json!({
                    "status": "skipped",
                    "reason": "not a connection event"
                })),
            }
        })
    }

    fn name(&self) -> &'static str {
        "connection_event_handler"
    }

    fn can_handle(&self, event: &crate::websocket::events::GenericWebSocketEvent) -> bool {
        event.event_type() == EventType::Connection
    }

    fn priority(&self) -> u32 {
        10 // 高优先级
    }
}

/// 消息事件处理器
pub struct MessageEventHandler;

impl EventHandler<crate::websocket::events::GenericWebSocketEvent> for MessageEventHandler {
    type Response = serde_json::Value;
    type Error = EventError;

    fn handle<'a>(
        &'a self,
        event: &'a crate::websocket::events::GenericWebSocketEvent,
        _ctx: &'a EventContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>,
    > {
        Box::pin(async move {
            use crate::websocket::events::{WebSocketEvent, WebSocketEventType};

            match &event.event_type {
                WebSocketEventType::Message {
                    message_type,
                    from_user_id,
                    to_user_id,
                    content,
                } => {
                    info!(
                        "Processing message from {} to {:?}: {:?}",
                        from_user_id, to_user_id, message_type
                    );
                    Ok(serde_json::json!({
                        "status": "processed",
                        "message_id": Uuid::new_v4(),
                        "from_user_id": from_user_id,
                        "to_user_id": to_user_id,
                        "message_type": format!("{:?}", message_type),
                        "processed_at": chrono::Utc::now()
                    }))
                }
                _ => Ok(serde_json::json!({
                    "status": "skipped",
                    "reason": "not a message event"
                })),
            }
        })
    }

    fn name(&self) -> &'static str {
        "message_event_handler"
    }

    fn can_handle(&self, event: &crate::websocket::events::GenericWebSocketEvent) -> bool {
        event.event_type() == EventType::Message
    }
}

/// 系统事件处理器
pub struct SystemEventHandler;

impl EventHandler<crate::websocket::events::GenericWebSocketEvent> for SystemEventHandler {
    type Response = serde_json::Value;
    type Error = EventError;

    fn handle<'a>(
        &'a self,
        event: &'a crate::websocket::events::GenericWebSocketEvent,
        _ctx: &'a EventContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>,
    > {
        Box::pin(async move {
            use crate::websocket::events::{WebSocketEvent, WebSocketEventType};

            match &event.event_type {
                WebSocketEventType::System {
                    system_event,
                    payload,
                } => {
                    info!("Processing system event: {:?}", system_event);
                    Ok(serde_json::json!({
                        "status": "processed",
                        "system_event": format!("{:?}", system_event),
                        "payload": payload,
                        "processed_at": chrono::Utc::now()
                    }))
                }
                _ => Ok(serde_json::json!({
                    "status": "skipped",
                    "reason": "not a system event"
                })),
            }
        })
    }

    fn name(&self) -> &'static str {
        "system_event_handler"
    }

    fn can_handle(&self, event: &crate::websocket::events::GenericWebSocketEvent) -> bool {
        event.event_type() == EventType::System
    }

    fn priority(&self) -> u32 {
        5 // 最高优先级
    }
}

/// 错误事件处理器
pub struct ErrorEventHandler;

impl EventHandler<crate::websocket::events::GenericWebSocketEvent> for ErrorEventHandler {
    type Response = serde_json::Value;
    type Error = EventError;

    fn handle<'a>(
        &'a self,
        event: &'a crate::websocket::events::GenericWebSocketEvent,
        ctx: &'a EventContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>,
    > {
        Box::pin(async move {
            use crate::websocket::events::WebSocketEvent;

            error!("Processing error event: {:?}", event);

            let error_details = serde_json::json!({
                "event_id": event.event_id(),
                "user_id": event.user_id,
                "connection_id": event.connection_id,
                "context_request_id": ctx.request_id,
                "timestamp": chrono::Utc::now(),
                "error_type": "event_processing_error"
            });

            Ok(serde_json::json!({
                "status": "error_handled",
                "error_details": error_details,
                "recovery_actions": ["logged", "notified"]
            }))
        })
    }

    fn name(&self) -> &'static str {
        "error_event_handler"
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    fn max_execution_time(&self) -> Duration {
        Duration::from_secs(5) // 错误处理应该快速完成
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::events::{ConnectionAction, EventBuilder, MessageEventType};

    #[tokio::test]
    async fn test_handler_registry() {
        let registry = HandlerRegistry::new();

        // 注册处理器
        registry
            .register::<crate::websocket::events::GenericWebSocketEvent>("test_handler")
            .await;

        let stats = registry.get_stats().await;
        assert_eq!(stats.total_handlers, 1);
    }

    #[tokio::test]
    async fn test_connection_event_handler() {
        let handler = ConnectionEventHandler;
        let event = EventBuilder::connection_event(
            ConnectionAction::Connect,
            Uuid::new_v4(),
            "test-connection".to_string(),
        );
        let ctx = EventContext::new();

        let result = handler.handle(&event, &ctx).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["status"], "processed");
    }

    #[tokio::test]
    async fn test_message_event_handler() {
        let handler = MessageEventHandler;
        let event = EventBuilder::message_event(
            MessageEventType::Text,
            Uuid::new_v4(),
            None,
            serde_json::json!({"text": "Hello World"}),
        );
        let ctx = EventContext::new();

        let result = handler.handle(&event, &ctx).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["status"], "processed");
    }
}
