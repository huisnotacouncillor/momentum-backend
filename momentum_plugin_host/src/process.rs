//! Plugin process management
//!
//! This module handles spawning and managing plugin processes.

use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::info;

pub struct ProcessManager;

impl ProcessManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn spawn_plugin(&self, plugin_id: &str, socket_path: &str) -> Result<Child, String> {
        info!("Spawning plugin {} with socket {}", plugin_id, socket_path);

        let child = Command::new("plugin-dummy")
            .arg("--socket-path")
            .arg(socket_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn plugin: {}", e))?;

        Ok(child)
    }

    pub async fn wait_for_plugin_ready(&self, socket_path: &str, timeout_secs: u64) -> Result<(), String> {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < timeout_secs {
            if tokio::fs::try_exists(socket_path).await.unwrap_or(false) {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Err("Plugin failed to become ready within timeout".to_string())
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
