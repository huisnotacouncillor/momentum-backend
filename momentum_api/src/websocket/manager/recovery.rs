use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use super::state::{ConnectionRecoveryInfo, ConnectedUser};

pub struct RecoveryManager {
    recovery_info: Arc<RwLock<HashMap<Uuid, ConnectionRecoveryInfo>>>,
    recovery_token_ttl: Duration,
}

impl RecoveryManager {
    pub fn new() -> Self {
        Self {
            recovery_info: Arc::new(RwLock::new(HashMap::new())),
            recovery_token_ttl: Duration::from_secs(300),
        }
    }

    pub async fn create_recovery_info(&self, user: &ConnectedUser) {
        let recovery_token = Uuid::new_v4().to_string();
        let expires_at =
            chrono::Utc::now() + chrono::Duration::from_std(self.recovery_token_ttl).unwrap();

        let recovery_info = ConnectionRecoveryInfo {
            user_id: user.user_id,
            recovery_token: recovery_token.clone(),
            expires_at,
            pending_messages: user.message_queue.clone(),
        };

        let mut recovery_map = self.recovery_info.write().await;
        recovery_map.insert(user.user_id, recovery_info);

        info!(
            "🔄 WebSocket Created recovery info for user {} with token {}",
            user.username, recovery_token
        );
    }

    pub async fn recover_connection(
        &self,
        user_id: Uuid,
        recovery_token: &str,
    ) -> Option<ConnectedUser> {
        let pending_messages = {
            let recovery_map = self.recovery_info.read().await;
            let info = recovery_map.get(&user_id)?;
            if info.recovery_token != recovery_token
                || info.expires_at <= chrono::Utc::now()
            {
                return None;
            }
            info.pending_messages.clone()
        };

        info!("🔄 WebSocket Recovered connection for user {}", user_id);
        Some(ConnectedUser {
            message_queue: pending_messages,
            ..Default::default()
        })
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}
