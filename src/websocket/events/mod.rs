//! 统一的WebSocket事件处理框架
//!
//! 这个模块提供了一个统一、可扩展的事件处理系统，用于处理各种类型的WebSocket事件。
//! 设计目标：
//! 1. 统一事件接收和处理流程
//! 2. 支持业务逻辑解耦
//! 3. 提供高性能的事件分发机制
//! 4. 支持中间件和插件扩展

pub mod business;
pub mod core;
pub mod handlers;
pub mod middleware;
pub mod types;

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// Re-export commonly used types
pub use business::{BusinessContext, BusinessEvent, BusinessEventHandler};
pub use core::{EventContext, EventDispatcher, EventError, EventResult};
pub use handlers::{
    ConnectionEventHandler, EventHandler, HandlerRegistry, MessageEventHandler, SystemEventHandler,
};
pub use middleware::{EventMiddleware, MiddlewareChain, create_default_middleware_chain};
pub use types::{EventMetadata, EventPriority, EventType};

/// 通用事件特征
pub trait Event: Send + Sync + fmt::Debug {
    /// 事件类型标识
    fn event_type(&self) -> EventType;

    /// 事件优先级
    fn priority(&self) -> EventPriority {
        EventPriority::Normal
    }

    /// 事件元数据
    fn metadata(&self) -> EventMetadata {
        EventMetadata::default()
    }

    /// 序列化事件数据
    fn serialize(&self) -> Result<serde_json::Value, EventError>;

    /// 事件验证
    fn validate(&self) -> Result<(), EventError> {
        Ok(())
    }

    /// 获取事件ID
    fn event_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// 获取时间戳
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

/// WebSocket特定的事件特征
pub trait WebSocketEvent: Event {
    /// 关联的用户ID
    fn user_id(&self) -> Option<Uuid>;

    /// 关联的连接ID
    fn connection_id(&self) -> Option<String>;

    /// 工作空间ID
    fn workspace_id(&self) -> Option<Uuid>;

    /// 是否需要广播
    fn should_broadcast(&self) -> bool {
        false
    }

    /// 广播目标用户
    fn broadcast_targets(&self) -> Vec<Uuid> {
        vec![]
    }

    /// 是否需要持久化
    fn should_persist(&self) -> bool {
        false
    }
}

/// 预定义的WebSocket事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSocketEventType {
    /// 连接事件
    Connection {
        action: ConnectionAction,
        user_id: Uuid,
        connection_id: String,
    },

    /// 消息事件
    Message {
        message_type: MessageEventType,
        from_user_id: Uuid,
        to_user_id: Option<Uuid>,
        content: serde_json::Value,
    },

    /// 业务命令事件
    Command {
        command_type: String,
        payload: serde_json::Value,
        request_id: Option<String>,
    },

    /// 系统事件
    System {
        system_event: SystemEventType,
        payload: serde_json::Value,
    },

    /// 自定义业务事件
    Business {
        business_type: String,
        payload: serde_json::Value,
        metadata: EventMetadata,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionAction {
    Connect,
    Disconnect,
    Reconnect,
    Suspend,
    Resume,
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageEventType {
    Text,
    Notification,
    Broadcast,
    Direct,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemEventType {
    UserJoined,
    UserLeft,
    WorkspaceChanged,
    ServerShutdown,
    MaintenanceMode,
}

/// 通用WebSocket事件实现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericWebSocketEvent {
    pub id: String,
    pub event_type: WebSocketEventType,
    pub user_id: Option<Uuid>,
    pub connection_id: Option<String>,
    pub workspace_id: Option<Uuid>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: EventMetadata,
    pub should_broadcast: bool,
    pub broadcast_targets: Vec<Uuid>,
    pub should_persist: bool,
}

impl Event for GenericWebSocketEvent {
    fn event_type(&self) -> EventType {
        match &self.event_type {
            WebSocketEventType::Connection { .. } => EventType::Connection,
            WebSocketEventType::Message { .. } => EventType::Message,
            WebSocketEventType::Command { .. } => EventType::Command,
            WebSocketEventType::System { .. } => EventType::System,
            WebSocketEventType::Business { .. } => EventType::Business,
        }
    }

    fn priority(&self) -> EventPriority {
        match &self.event_type {
            WebSocketEventType::System { .. } => EventPriority::High,
            WebSocketEventType::Connection { .. } => EventPriority::High,
            WebSocketEventType::Command { .. } => EventPriority::Normal,
            WebSocketEventType::Message { .. } => EventPriority::Normal,
            WebSocketEventType::Business { .. } => EventPriority::Normal,
        }
    }

    fn metadata(&self) -> EventMetadata {
        self.metadata.clone()
    }

    fn serialize(&self) -> Result<serde_json::Value, EventError> {
        serde_json::to_value(self).map_err(|e| EventError::SerializationError(e.to_string()))
    }

    fn event_id(&self) -> String {
        self.id.clone()
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }
}

impl WebSocketEvent for GenericWebSocketEvent {
    fn user_id(&self) -> Option<Uuid> {
        self.user_id
    }

    fn connection_id(&self) -> Option<String> {
        self.connection_id.clone()
    }

    fn workspace_id(&self) -> Option<Uuid> {
        self.workspace_id
    }

    fn should_broadcast(&self) -> bool {
        self.should_broadcast
    }

    fn broadcast_targets(&self) -> Vec<Uuid> {
        self.broadcast_targets.clone()
    }

    fn should_persist(&self) -> bool {
        self.should_persist
    }
}

/// 事件构建器，提供便利的事件创建方法
pub struct EventBuilder;

impl EventBuilder {
    /// 创建连接事件
    pub fn connection_event(
        action: ConnectionAction,
        user_id: Uuid,
        connection_id: String,
    ) -> GenericWebSocketEvent {
        GenericWebSocketEvent {
            id: Uuid::new_v4().to_string(),
            event_type: WebSocketEventType::Connection {
                action,
                user_id,
                connection_id: connection_id.clone(),
            },
            user_id: Some(user_id),
            connection_id: Some(connection_id),
            workspace_id: None,
            timestamp: chrono::Utc::now(),
            metadata: EventMetadata::default(),
            should_broadcast: true,
            broadcast_targets: vec![],
            should_persist: false,
        }
    }

    /// 创建消息事件
    pub fn message_event(
        message_type: MessageEventType,
        from_user_id: Uuid,
        to_user_id: Option<Uuid>,
        content: serde_json::Value,
    ) -> GenericWebSocketEvent {
        let should_broadcast = to_user_id.is_none();
        let broadcast_targets = if let Some(target) = to_user_id {
            vec![target]
        } else {
            vec![]
        };

        GenericWebSocketEvent {
            id: Uuid::new_v4().to_string(),
            event_type: WebSocketEventType::Message {
                message_type,
                from_user_id,
                to_user_id,
                content,
            },
            user_id: Some(from_user_id),
            connection_id: None,
            workspace_id: None,
            timestamp: chrono::Utc::now(),
            metadata: EventMetadata::default(),
            should_broadcast,
            broadcast_targets,
            should_persist: false,
        }
    }

    /// 创建业务事件
    pub fn business_event(
        business_type: String,
        payload: serde_json::Value,
        user_id: Option<Uuid>,
        workspace_id: Option<Uuid>,
    ) -> GenericWebSocketEvent {
        GenericWebSocketEvent {
            id: Uuid::new_v4().to_string(),
            event_type: WebSocketEventType::Business {
                business_type,
                payload,
                metadata: EventMetadata::default(),
            },
            user_id,
            connection_id: None,
            workspace_id,
            timestamp: chrono::Utc::now(),
            metadata: EventMetadata::default(),
            should_broadcast: false,
            broadcast_targets: vec![],
            should_persist: false,
        }
    }

    /// 创建系统事件
    pub fn system_event(
        system_event: SystemEventType,
        payload: serde_json::Value,
    ) -> GenericWebSocketEvent {
        GenericWebSocketEvent {
            id: Uuid::new_v4().to_string(),
            event_type: WebSocketEventType::System {
                system_event,
                payload,
            },
            user_id: None,
            connection_id: None,
            workspace_id: None,
            timestamp: chrono::Utc::now(),
            metadata: EventMetadata::default(),
            should_broadcast: true,
            broadcast_targets: vec![],
            should_persist: false,
        }
    }
}

/// WebSocket事件系统，集成到现有的WebSocket管理器中
pub struct WebSocketEventSystem {
    dispatcher: EventDispatcher,
}

impl WebSocketEventSystem {
    pub fn new() -> Self {
        Self {
            dispatcher: EventDispatcher::new(),
        }
    }

    /// 分发事件
    pub async fn dispatch(&self, event: GenericWebSocketEvent) -> Result<EventResult, EventError> {
        self.dispatcher.dispatch(event).await
    }

    /// 批量分发事件
    pub async fn dispatch_batch(
        &self,
        events: Vec<GenericWebSocketEvent>,
    ) -> Vec<Result<EventResult, EventError>> {
        let mut results = Vec::new();
        for event in events {
            results.push(self.dispatch(event).await);
        }
        results
    }
}

impl Default for WebSocketEventSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_builder() {
        let user_id = Uuid::new_v4();
        let connection_id = "test-connection".to_string();

        let event = EventBuilder::connection_event(
            ConnectionAction::Connect,
            user_id,
            connection_id.clone(),
        );

        assert_eq!(event.user_id(), Some(user_id));
        assert_eq!(event.connection_id(), Some(connection_id));
        assert_eq!(event.event_type(), EventType::Connection);
    }

    #[tokio::test]
    async fn test_event_serialization() {
        let event = EventBuilder::message_event(
            MessageEventType::Text,
            Uuid::new_v4(),
            None,
            serde_json::json!({"text": "Hello World"}),
        );

        let serialized = event.serialize().unwrap();
        assert!(serialized.is_object());
    }

    #[test]
    fn test_event_priority() {
        let system_event =
            EventBuilder::system_event(SystemEventType::UserJoined, serde_json::json!({}));

        let message_event = EventBuilder::message_event(
            MessageEventType::Text,
            Uuid::new_v4(),
            None,
            serde_json::json!({"text": "test"}),
        );

        assert_eq!(system_event.priority(), EventPriority::High);
        assert_eq!(message_event.priority(), EventPriority::Normal);
    }
}
