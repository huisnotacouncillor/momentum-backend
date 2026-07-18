use std::collections::VecDeque;
use std::sync::Arc;
use uuid::Uuid;

use super::connection::ConnectionManager;
use super::state::{ConnectedUser, WebSocketMessage};

pub struct OfflineQueueManager {
    conn: Arc<ConnectionManager>,
    max_queue_size: usize,
}

impl OfflineQueueManager {
    pub fn new(conn: Arc<ConnectionManager>) -> Self {
        Self { conn, max_queue_size: 100 }
    }

    pub async fn add_offline_message(&self, user_id: Uuid, message: WebSocketMessage) {
        if let Some(mut user) = self.conn.get_user_mut(user_id).await {
            user.message_queue.push_back(message);
            if user.message_queue.len() > self.max_queue_size {
                user.message_queue.pop_front();
            }
        }
    }

    pub async fn get_offline_messages(&self, user_id: Uuid) -> VecDeque<WebSocketMessage> {
        if let Some(mut user) = self.conn.get_user_mut(user_id).await {
            let messages = user.message_queue.clone();
            user.message_queue.clear();
            messages
        } else {
            VecDeque::new()
        }
    }
}
