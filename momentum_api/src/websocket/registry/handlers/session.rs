//! SubscriptionSession — 轻量 connection scope 抽象
//!
//! Step 7 引入：SubscribeHandler / UnsubscribeHandler 通过
//! SubscriptionSession 间接访问 SubscriptionManager。
//!
//! 设计目的：
//! - Handler 不直接持有 Arc<SubscriptionManager>，方便单测注入 mock session
//! - 连接级 connection_id 与 Manager 解耦；Step 8+ 由 axum WebSocket handler
//!   在 dispatch 时构造一次 session 即可。

use std::sync::Arc;

use crate::websocket::manager::subscription::{
    SubscribeResult, SubscriptionManager, UnsubscribeResult,
};

#[derive(Clone)]
pub struct SubscriptionSession {
    manager: Arc<SubscriptionManager>,
    connection_id: String,
}

impl SubscriptionSession {
    pub fn new(manager: Arc<SubscriptionManager>, connection_id: impl Into<String>) -> Self {
        Self {
            manager,
            connection_id: connection_id.into(),
        }
    }

    pub async fn subscribe(&self, topics: &[crate::websocket::manager::subscription::Topic]) -> SubscribeResult {
        self.manager.subscribe(&self.connection_id, topics).await
    }

    pub async fn unsubscribe(&self, topics: &[crate::websocket::manager::subscription::Topic]) -> UnsubscribeResult {
        self.manager.unsubscribe(&self.connection_id, topics).await
    }

    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    /// 仅测试 / 调试使用：从 session 拿到共享 SubscriptionManager
    pub fn manager(&self) -> &Arc<SubscriptionManager> {
        &self.manager
    }
}
