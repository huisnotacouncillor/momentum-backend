//! Plugin 审计日志
//!
//! 记录所有插件相关事件：安装、启用、字段写入、Agent 调用、权限拒绝、错误等
//! 详见 docs/PLUGIN_SDK_DESIGN.md

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuditEvent {
    pub plugin_id: String,
    pub workspace_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub event: String,
    pub payload: Option<serde_json::Value>,
}

/// 审计事件类型常量（便于全文搜索和统计）
pub mod events {
    pub const INSTALLED: &str = "installed";
    pub const ENABLED: &str = "enabled";
    pub const DISABLED: &str = "disabled";
    pub const UPGRADED: &str = "upgraded";
    pub const UNINSTALLED: &str = "uninstalled";
    pub const FIELD_SET: &str = "field.set";
    pub const AGENT_INVOKED: &str = "agent.invoked";
    pub const PERMISSION_DENIED: &str = "permission_denied";
    pub const ERROR: &str = "error";
    pub const HANDSHAKE: &str = "handshake";
    /// 模板：插件 publish 事件时用 `format!("event.publish.{}", event_type)`
    pub const EVENT_PUBLISH: &str = "event.publish.{event_type}";
}
