//! Registry 内存状态
//!
//! 完整的 enable/disable + gRPC client 缓存在 momentum_plugin_host crate。
//! 这里只放最简的 PluginInstance struct + RegistryState。

use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    Installed,
    Enabled,
    Disabled,
    Error,
}

/// 插件运行实例（内存）
#[derive(Debug, Clone)]
pub struct PluginInstance {
    pub plugin_id: String,
    pub workspace_id: Uuid,
    pub status: PluginStatus,
    /// gRPC client 缓存（v0.2 接入）
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Default)]
pub struct RegistryState {
    pub instances: HashMap<(String, Uuid), PluginInstance>,
}

impl RegistryState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, plugin_id: &str, workspace_id: Uuid) -> Option<&PluginInstance> {
        self.instances.get(&(plugin_id.to_string(), workspace_id))
    }

    pub fn insert(&mut self, inst: PluginInstance) {
        let key = (inst.plugin_id.clone(), inst.workspace_id);
        self.instances.insert(key, inst);
    }

    pub fn remove(&mut self, plugin_id: &str, workspace_id: Uuid) -> Option<PluginInstance> {
        self.instances
            .remove(&(plugin_id.to_string(), workspace_id))
    }

    pub fn list(&self) -> Vec<&PluginInstance> {
        self.instances.values().collect()
    }
}
