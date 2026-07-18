use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use super::state::{ConnectedUser, ConnectionState};

pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, ConnectedUser>>>,
    #[allow(dead_code)]
    max_queue_size: usize,  // Reserved for future use (e.g., per-user queue limits)
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_queue_size: 100,
        }
    }

    pub async fn add_connection(
        &self,
        connection_id: String,
        user: ConnectedUser,
        _db: Option<&Arc<momentum_core::db::DbPool>>,
        _asset_helper: Option<&Arc<momentum_core::utils::AssetUrlHelper>>,
    ) {
        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), user.clone());
        info!(
            "🔌 WebSocket User {} connected with connection ID {}",
            user.username, connection_id
        );
    }

    pub async fn remove_connection(&self, connection_id: &str) -> Option<ConnectedUser> {
        let mut connections = self.connections.write().await;
        connections.remove(connection_id)
    }

    pub async fn suspend_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(connection_id) {
            user.state = ConnectionState::Suspended;
            info!("⏸️ WebSocket Suspended connection for user {}", user.username);
        }
    }

    pub async fn resume_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(connection_id) {
            user.state = ConnectionState::Connected;
            info!("▶️ WebSocket Resumed connection for user {}", user.username);
        }
    }

    pub async fn get_connection(&self, connection_id: &str) -> Option<ConnectedUser> {
        let connections = self.connections.read().await;
        connections.get(connection_id).cloned()
    }

    pub async fn update_ping(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(connection_id) {
            user.last_ping = chrono::Utc::now();
        }
    }

    pub async fn get_online_users(&self) -> Vec<ConnectedUser> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    pub async fn get_connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    pub async fn get_user_mut(&self, user_id: Uuid) -> Option<ConnectedUser> {
        let connections = self.connections.read().await;
        for (_, user) in connections.iter() {
            if user.user_id == user_id {
                return Some(user.clone());
            }
        }
        None
    }

    pub async fn get_connections_in_workspace(&self, workspace_id: Uuid) -> Vec<String> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter(|(_, user)| user.current_workspace_id == Some(workspace_id))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub async fn get_connections_by_user(&self, user_id: Uuid) -> Vec<String> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter(|(_, user)| user.user_id == user_id)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub async fn cleanup_stale_connections(&self, timeout_minutes: i64) -> usize {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(timeout_minutes);
        let mut connections = self.connections.write().await;
        let stale: Vec<_> = connections
            .iter()
            .filter(|(_, user)| user.last_ping < cutoff)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &stale {
            connections.remove(id);
        }
        stale.len()
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
