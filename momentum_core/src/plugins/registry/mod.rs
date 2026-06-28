//! Plugin Registry（生命周期 + 内存状态）
//!
//! v0.1：纯内存 HashMap（重启后从 DB 恢复 installed/disabled 状态，
//      enabled 状态需要 plugin_host 重连）
//!
//! 完整实现在 momentum_plugin_host crate（进程管理 + gRPC client 缓存）。
//!
//! 详见 docs/PLUGIN_SDK_DESIGN.md §7

pub mod state;

pub use state::*;

use std::sync::Arc;
use tokio::sync::RwLock;

/// 顶层 Plugin Registry（运行时）
pub struct PluginRegistry {
    state: Arc<RwLock<RegistryState>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(RegistryState::default())),
        }
    }

    /// 获取共享 state 句柄
    pub fn state(&self) -> Arc<RwLock<RegistryState>> {
        self.state.clone()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
