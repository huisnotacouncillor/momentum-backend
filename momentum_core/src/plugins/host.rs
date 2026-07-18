//! Plugin Host Trait
//!
//! 定义插件主机的抽象接口，支持：
//! - 进程生命周期管理（spawn/terminate）
//! - gRPC 命令发送
//! - 健康检查
//!
//! ## 设计原则
//!
//! - **接口抽象**：通过 trait 解耦，允许不同实现（本地/远程/NoOp）
//! - **Feature Flag**：`#[cfg(feature = "plugins")]` 控制插件功能编译
//! - **向后兼容**：NoOpPluginHost 作为默认实现，不影响现有功能
//!
//! ## 使用方式
//!
//! ```rust,ignore
//! // 定义服务
//! pub struct MyService {
//!     plugin_host: Arc<dyn PluginHost>,
//! }
//!
//! // 安装插件时调用
//! impl MyService {
//!     pub async fn install(&self, plugin_id: Uuid) -> Result<()> {
//!         self.plugin_host.spawn(plugin_id).await?;
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::{PluginError, PluginResult};

/// 插件主机接口
///
/// 通过 `Arc<dyn PluginHost>` 注入到服务中。
/// 默认实现为 NoOpPluginHost（所有操作都是空操作）。
#[async_trait::async_trait]
pub trait PluginHost: Send + Sync {
    // ============================================================
    // 进程生命周期管理
    // ============================================================

    /// 启动插件进程
    ///
    /// 具体的启动方式由实现决定：
    /// - NoOp: 什么都不做
    /// - GrpcHost: spawn 子进程并建立 gRPC 连接
    async fn spawn(&self, plugin_id: Uuid) -> PluginResult<()>;

    /// 停止插件进程
    async fn terminate(&self, plugin_id: Uuid) -> PluginResult<()>;

    // ============================================================
    // gRPC 命令交互
    // ============================================================

    /// 发送命令给插件
    async fn send_command(
        &self,
        plugin_id: Uuid,
        command: PluginCommand,
    ) -> PluginResult<PluginResponse>;

    // ============================================================
    // 健康检查
    // ============================================================

    /// 检查插件是否存活
    async fn health_check(&self, plugin_id: Uuid) -> PluginResult<bool>;
}

// ============================================================
// PluginCommand / PluginResponse
// ============================================================

/// 发送给插件的命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    /// 命令类型
    pub command_type: String,
    /// 命令参数（JSON）
    pub payload: serde_json::Value,
}

impl PluginCommand {
    pub fn new(command_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            command_type: command_type.into(),
            payload,
        }
    }
}

/// 插件响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// 是否成功
    pub success: bool,
    /// 响应数据
    pub data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
}

impl PluginResponse {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

// ============================================================
// NoOpPluginHost - 默认实现（空操作）
// ============================================================

/// 空实现插件主机
///
/// 所有操作都是空操作，用于：
/// - 插件功能未启用时
/// - 测试环境
/// - 隔离依赖
pub struct NoOpPluginHost;

impl NoOpPluginHost {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpPluginHost {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PluginHost for NoOpPluginHost {
    async fn spawn(&self, _plugin_id: Uuid) -> PluginResult<()> {
        // NoOp: 什么都不做
        Ok(())
    }

    async fn terminate(&self, _plugin_id: Uuid) -> PluginResult<()> {
        // NoOp: 什么都不做
        Ok(())
    }

    async fn send_command(
        &self,
        _plugin_id: Uuid,
        _command: PluginCommand,
    ) -> PluginResult<PluginResponse> {
        // NoOp: 返回错误（插件系统未启用）
        Err(PluginError::Other(
            "plugin system not enabled".into(),
        ))
    }

    async fn health_check(&self, _plugin_id: Uuid) -> PluginResult<bool> {
        // NoOp: 永远返回 false
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_spawn_terminates() {
        let host = NoOpPluginHost::new();
        let plugin_id = Uuid::new_v4();

        // spawn 应该成功（空操作）
        assert!(host.spawn(plugin_id).await.is_ok());

        // terminate 应该成功（空操作）
        assert!(host.terminate(plugin_id).await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_send_command_fails() {
        let host = NoOpPluginHost::new();
        let plugin_id = Uuid::new_v4();
        let cmd = PluginCommand::new("test", serde_json::json!({}));

        let result = host.send_command(plugin_id, cmd).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_noop_health_check_returns_false() {
        let host = NoOpPluginHost::new();
        let plugin_id = Uuid::new_v4();

        let healthy = host.health_check(plugin_id).await.unwrap();
        assert!(!healthy);
    }

    #[test]
    fn test_plugin_command_new() {
        let cmd = PluginCommand::new("echo", serde_json::json!({"msg": "hello"}));
        assert_eq!(cmd.command_type, "echo");
        assert_eq!(cmd.payload, serde_json::json!({"msg": "hello"}));
    }

    #[test]
    fn test_plugin_response_ok() {
        let resp = PluginResponse::ok(serde_json::json!({"result": "ok"}));
        assert!(resp.success);
        assert!(resp.error.is_none());
        assert_eq!(resp.data, Some(serde_json::json!({"result": "ok"})));
    }

    #[test]
    fn test_plugin_response_err() {
        let resp = PluginResponse::err("something went wrong");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert_eq!(resp.error, Some("something went wrong".into()));
    }
}
