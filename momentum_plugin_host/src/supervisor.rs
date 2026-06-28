//! Plugin supervisor
//!
//! This module supervises plugin processes and handles their lifecycle.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::process::ProcessManager;

#[derive(Debug, Clone)]
pub struct PluginInstance {
    pub plugin_id: String,
    pub socket_path: String,
}

pub struct Supervisor {
    instances: Arc<RwLock<HashMap<String, PluginInstance>>>,
    process_manager: ProcessManager,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            process_manager: ProcessManager::new(),
        }
    }

    pub async fn start_plugin(&self, plugin_id: &str, socket_path: &str) -> Result<(), String> {
        info!("Starting plugin supervisor for {}", plugin_id);

        let _child = self.process_manager.spawn_plugin(plugin_id, socket_path).await?;

        let instance = PluginInstance {
            plugin_id: plugin_id.to_string(),
            socket_path: socket_path.to_string(),
        };

        let mut instances = self.instances.write().await;
        instances.insert(plugin_id.to_string(), instance);

        Ok(())
    }

    pub async fn stop_plugin(&self, plugin_id: &str) -> Result<(), String> {
        info!("Stopping plugin {}", plugin_id);

        let mut instances = self.instances.write().await;
        instances.remove(plugin_id);

        Ok(())
    }

    pub async fn list_plugins(&self) -> Vec<String> {
        let instances = self.instances.read().await;
        instances.keys().cloned().collect()
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}
