pub mod basic_tests;
pub mod stress_tests;

// Common test utilities and helper functions
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

/// Common test configuration
#[allow(dead_code)]
pub struct TestConfig {
    pub websocket_url: String,
    pub jwt_secret: String,
    pub timeout_duration: Duration,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            websocket_url: "ws://127.0.0.1:8000/ws".to_string(),
            jwt_secret: "test_jwt_secret_key".to_string(),
            timeout_duration: Duration::from_secs(5),
        }
    }
}

/// Helper function to create test JWT tokens
pub fn create_test_jwt(user_id: Uuid, username: &str, secret: &str) -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    #[derive(serde::Serialize)]
    struct TestClaims {
        sub: String,
        username: String,
        exp: usize,
        iat: usize,
    }

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = TestClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: now + 3600, // 1 hour from now
        iat: now,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .unwrap()
}

/// Helper function to create test WebSocket message
pub fn create_test_message(
    message_type: rust_backend::websocket::MessageType,
    content: &str,
    from_user_id: Option<Uuid>,
    to_user_id: Option<Uuid>,
) -> rust_backend::websocket::WebSocketMessage {
    rust_backend::websocket::WebSocketMessage {
        id: Uuid::new_v4().to_string(),
        message_type,
        data: json!({
            "content": content,
            "timestamp": chrono::Utc::now()
        }),
        timestamp: chrono::Utc::now(),
        from_user_id,
        to_user_id,
        secure_message: None,
    }
}

/// Helper function to create test connected user
pub fn create_test_connected_user(username: &str) -> rust_backend::websocket::ConnectedUser {
    rust_backend::websocket::ConnectedUser {
        user_id: Uuid::new_v4(),
        username: username.to_string(),
        connected_at: chrono::Utc::now(),
        last_ping: chrono::Utc::now(),
        state: rust_backend::websocket::manager::ConnectionState::Connected,
        subscriptions: std::collections::HashSet::new(),
        message_queue: std::collections::VecDeque::new(),
        recovery_token: None,
        metadata: std::collections::HashMap::new(),
    }
}

/// Wait for a condition to be true with timeout
pub async fn wait_for_condition<F, Fut>(
    mut condition: F,
    timeout: Duration,
    check_interval: Duration,
) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(check_interval).await;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_jwt() {
        let user_id = Uuid::new_v4();
        let username = "test_user";
        let secret = "test_secret";

        let token = create_test_jwt(user_id, username, secret);

        // Basic JWT format validation
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(!parts[0].is_empty());
        assert!(!parts[1].is_empty());
        assert!(!parts[2].is_empty());
    }

    #[test]
    fn test_create_test_message() {
        let message = create_test_message(
            rust_backend::websocket::MessageType::Text,
            "Hello, World!",
            Some(Uuid::new_v4()),
            None,
        );

        assert!(!message.id.is_empty());
        assert_eq!(message.data["content"], "Hello, World!");
        assert!(message.from_user_id.is_some());
        assert!(message.to_user_id.is_none());
    }

    #[test]
    fn test_create_test_connected_user() {
        let user = create_test_connected_user("test_user");

        assert_eq!(user.username, "test_user");
        assert!(user.connected_at <= chrono::Utc::now());
        assert!(user.last_ping <= chrono::Utc::now());
    }

    #[tokio::test]
    async fn test_wait_for_condition() {
        let mut counter = 0;

        let result = wait_for_condition(
            || {
                counter += 1;
                async move { counter >= 3 }
            },
            Duration::from_millis(100),
            Duration::from_millis(10),
        )
        .await;

        assert!(result);
        assert!(counter >= 3);
    }

    #[tokio::test]
    async fn test_wait_for_condition_timeout() {
        let result = wait_for_condition(
            || async { false }, // Always false
            Duration::from_millis(50),
            Duration::from_millis(10),
        )
        .await;

        assert!(!result);
    }
}
