use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use url::Url;
use uuid::Uuid;

use rust_backend::websocket::{
    auth::WebSocketAuth,
    manager::{MessageType, WebSocketManager, WebSocketMessage},
};

const TEST_JWT_SECRET: &str = "test_jwt_secret_key";
const WEBSOCKET_URL: &str = "ws://127.0.0.1:8000/ws";

#[tokio::test]
async fn test_websocket_manager_creation() {
    let manager = WebSocketManager::new();
    let count = manager.get_connection_count().await;
    assert_eq!(count, 0);

    let users = manager.get_online_users().await;
    assert!(users.is_empty());
}

#[tokio::test]
async fn test_websocket_message_serialization() {
    let message = WebSocketMessage {
        id: "test_id".to_string(),
        message_type: MessageType::Text,
        data: json!({"content": "Hello, World!"}),
        timestamp: chrono::Utc::now(),
        from_user_id: Some(Uuid::new_v4()),
        to_user_id: None,
    };

    let serialized = serde_json::to_string(&message).unwrap();
    let deserialized: WebSocketMessage = serde_json::from_str(&serialized).unwrap();

    assert_eq!(message.id, deserialized.id);
    assert_eq!(message.data, deserialized.data);
    match (message.message_type, deserialized.message_type) {
        (MessageType::Text, MessageType::Text) => (),
        _ => panic!("Message types don't match"),
    }
}

#[tokio::test]
async fn test_websocket_auth_token_validation() {
    // Test valid JWT format
    let valid_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    assert!(WebSocketAuth::validate_token_format(valid_token));

    // Test invalid JWT format
    let invalid_token = "invalid.token";
    assert!(!WebSocketAuth::validate_token_format(invalid_token));

    let empty_token = "";
    assert!(!WebSocketAuth::validate_token_format(empty_token));

    let single_part = "onlyonepart";
    assert!(!WebSocketAuth::validate_token_format(single_part));
}

#[tokio::test]
async fn test_websocket_manager_broadcast() {
    let manager = WebSocketManager::new();
    let mut rx = manager.subscribe();

    let test_message = WebSocketMessage {
        id: Uuid::new_v4().to_string(),
        message_type: MessageType::Text,
        data: json!({"content": "Test broadcast message"}),
        timestamp: chrono::Utc::now(),
        from_user_id: Some(Uuid::new_v4()),
        to_user_id: None,
    };

    // Broadcast the message
    manager.broadcast_message(test_message.clone()).await;

    // Try to receive the message with timeout
    let received = timeout(Duration::from_millis(100), rx.recv()).await;

    match received {
        Ok(Ok(message)) => {
            assert_eq!(message.id, test_message.id);
            assert_eq!(message.data, test_message.data);
        }
        _ => panic!("Failed to receive broadcasted message"),
    }
}

#[tokio::test]
async fn test_websocket_manager_connection_lifecycle() {
    use rust_backend::websocket::manager::ConnectedUser;

    let manager = WebSocketManager::new();
    let connection_id = "test_connection_123".to_string();
    let user_id = Uuid::new_v4();

    let user = ConnectedUser {
        user_id,
        username: "test_user".to_string(),
        connected_at: chrono::Utc::now(),
        last_ping: chrono::Utc::now(),
    };

    // Initially no connections
    assert_eq!(manager.get_connection_count().await, 0);

    // Add connection
    manager
        .add_connection(connection_id.clone(), user.clone())
        .await;
    assert_eq!(manager.get_connection_count().await, 1);

    // Get connection
    let retrieved_user = manager.get_connection(&connection_id).await;
    assert!(retrieved_user.is_some());
    assert_eq!(retrieved_user.unwrap().user_id, user_id);

    // Update ping
    let old_ping = user.last_ping;
    tokio::time::sleep(Duration::from_millis(10)).await;
    manager.update_ping(&connection_id).await;

    let updated_user = manager.get_connection(&connection_id).await.unwrap();
    assert!(updated_user.last_ping > old_ping);

    // Remove connection
    manager.remove_connection(&connection_id).await;
    assert_eq!(manager.get_connection_count().await, 0);
}

#[tokio::test]
async fn test_websocket_manager_cleanup_stale_connections() {
    use rust_backend::websocket::manager::ConnectedUser;

    let manager = WebSocketManager::new();
    let connection_id = "stale_connection".to_string();
    let user_id = Uuid::new_v4();

    // Create a user with old ping time
    let old_time = chrono::Utc::now() - chrono::Duration::minutes(15);
    let stale_user = ConnectedUser {
        user_id,
        username: "stale_user".to_string(),
        connected_at: old_time,
        last_ping: old_time,
    };

    manager
        .add_connection(connection_id.clone(), stale_user)
        .await;
    assert_eq!(manager.get_connection_count().await, 1);

    // Cleanup connections older than 10 minutes
    manager.cleanup_stale_connections(10).await;
    assert_eq!(manager.get_connection_count().await, 0);
}

#[tokio::test]
async fn test_websocket_message_types() {
    let test_cases = vec![
        (MessageType::Text, "text"),
        (MessageType::Notification, "notification"),
        (MessageType::SystemMessage, "system_message"),
        (MessageType::UserJoined, "user_joined"),
        (MessageType::UserLeft, "user_left"),
        (MessageType::Ping, "ping"),
        (MessageType::Pong, "pong"),
        (MessageType::Error, "error"),
    ];

    for (message_type, expected_str) in test_cases {
        let message = WebSocketMessage {
            id: Uuid::new_v4().to_string(),
            message_type: message_type.clone(),
            data: json!({}),
            timestamp: chrono::Utc::now(),
            from_user_id: None,
            to_user_id: None,
        };

        let serialized = serde_json::to_string(&message).unwrap();
        assert!(serialized.contains(expected_str));

        let deserialized: WebSocketMessage = serde_json::from_str(&serialized).unwrap();
        // We can't directly compare MessageType enum variants, so we serialize both and compare
        let original_type_str = serde_json::to_string(&message.message_type).unwrap();
        let deserialized_type_str = serde_json::to_string(&deserialized.message_type).unwrap();
        assert_eq!(original_type_str, deserialized_type_str);
    }
}

#[tokio::test]
async fn test_websocket_manager_send_to_user() {
    use rust_backend::websocket::manager::ConnectedUser;

    let manager = WebSocketManager::new();
    let user_id = Uuid::new_v4();
    let connection_id = "user_connection".to_string();

    let user = ConnectedUser {
        user_id,
        username: "target_user".to_string(),
        connected_at: chrono::Utc::now(),
        last_ping: chrono::Utc::now(),
    };

    manager.add_connection(connection_id, user).await;

    let test_message = WebSocketMessage {
        id: Uuid::new_v4().to_string(),
        message_type: MessageType::Text,
        data: json!({"content": "Direct message"}),
        timestamp: chrono::Utc::now(),
        from_user_id: Some(Uuid::new_v4()),
        to_user_id: Some(user_id),
    };

    // This should not fail (we're just testing the API)
    manager.send_to_user(user_id, test_message).await;
}

#[tokio::test]
async fn test_websocket_manager_multiple_users() {
    use rust_backend::websocket::manager::ConnectedUser;

    let manager = WebSocketManager::new();
    let mut users = Vec::new();

    // Add multiple users
    for i in 0..5 {
        let user_id = Uuid::new_v4();
        let connection_id = format!("connection_{}", i);
        let user = ConnectedUser {
            user_id,
            username: format!("user_{}", i),
            connected_at: chrono::Utc::now(),
            last_ping: chrono::Utc::now(),
        };

        users.push((connection_id.clone(), user_id));
        manager.add_connection(connection_id, user).await;
    }

    assert_eq!(manager.get_connection_count().await, 5);

    let online_users = manager.get_online_users().await;
    assert_eq!(online_users.len(), 5);

    // Remove users one by one
    for (connection_id, _) in users {
        manager.remove_connection(&connection_id).await;
    }

    assert_eq!(manager.get_connection_count().await, 0);
}

#[tokio::test]
async fn test_websocket_error_handling() {
    let manager = WebSocketManager::new();

    // Test getting non-existent connection
    let result = manager.get_connection("non_existent").await;
    assert!(result.is_none());

    // Test removing non-existent connection (should not panic)
    manager.remove_connection("non_existent").await;

    // Test updating ping for non-existent connection (should not panic)
    manager.update_ping("non_existent").await;
}

// Integration test helper functions
fn create_test_jwt(user_id: Uuid, username: &str, secret: &str) -> String {
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

#[tokio::test]
async fn test_jwt_token_creation() {
    let user_id = Uuid::new_v4();
    let username = "test_user";
    let secret = "test_secret";

    let token = create_test_jwt(user_id, username, secret);
    assert!(WebSocketAuth::validate_token_format(&token));

    // Token should have 3 parts separated by dots
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert!(!parts[0].is_empty()); // header
    assert!(!parts[1].is_empty()); // payload
    assert!(!parts[2].is_empty()); // signature
}

// Performance and stress tests
#[tokio::test]
async fn test_websocket_manager_performance() {
    use rust_backend::websocket::manager::ConnectedUser;

    let manager = WebSocketManager::new();
    let start_time = std::time::Instant::now();

    // Add 100 connections quickly
    for i in 0..100 {
        let user_id = Uuid::new_v4();
        let connection_id = format!("perf_connection_{}", i);
        let user = ConnectedUser {
            user_id,
            username: format!("perf_user_{}", i),
            connected_at: chrono::Utc::now(),
            last_ping: chrono::Utc::now(),
        };

        manager.add_connection(connection_id, user).await;
    }

    let add_duration = start_time.elapsed();
    assert!(add_duration < Duration::from_millis(1000)); // Should complete within 1 second

    assert_eq!(manager.get_connection_count().await, 100);

    // Test broadcast performance
    let broadcast_start = std::time::Instant::now();
    let test_message = WebSocketMessage {
        id: Uuid::new_v4().to_string(),
        message_type: MessageType::Text,
        data: json!({"content": "Performance test broadcast"}),
        timestamp: chrono::Utc::now(),
        from_user_id: Some(Uuid::new_v4()),
        to_user_id: None,
    };

    manager.broadcast_message(test_message).await;
    let broadcast_duration = broadcast_start.elapsed();
    assert!(broadcast_duration < Duration::from_millis(100)); // Should complete quickly

    // Cleanup performance test
    let cleanup_start = std::time::Instant::now();
    manager.cleanup_stale_connections(0).await; // Clean all connections
    let cleanup_duration = cleanup_start.elapsed();
    assert!(cleanup_duration < Duration::from_millis(500));
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // These tests would require a running server instance
    // They are marked as ignored by default and can be run with --ignored flag

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_websocket_connection_with_valid_token() {
        let user_id = Uuid::new_v4();
        let token = create_test_jwt(user_id, "test_user", TEST_JWT_SECRET);
        let url = format!("{}?token={}", WEBSOCKET_URL, token);

        let url = Url::parse(&url).unwrap();
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Send a ping message
        let ping_message = json!({
            "id": Uuid::new_v4().to_string(),
            "message_type": "ping",
            "data": {},
            "timestamp": chrono::Utc::now(),
        });

        ws_sender
            .send(TungsteniteMessage::Text(ping_message.to_string()))
            .await
            .expect("Failed to send ping");

        // Wait for response
        if let Ok(Some(msg)) = timeout(Duration::from_secs(5), ws_receiver.next()).await {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    let response: serde_json::Value = serde_json::from_str(&text).unwrap();
                    // Should receive welcome message or pong
                    assert!(response.get("message_type").is_some());
                }
                _ => panic!("Unexpected message type"),
            }
        } else {
            panic!("No response received within timeout");
        }
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_websocket_connection_without_token() {
        let url = Url::parse(WEBSOCKET_URL).unwrap();

        // This should fail due to missing authentication
        let result = connect_async(url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_websocket_connection_with_invalid_token() {
        let invalid_token = "invalid.jwt.token";
        let url = format!("{}?token={}", WEBSOCKET_URL, invalid_token);

        let url = Url::parse(&url).unwrap();
        let result = connect_async(url).await;
        assert!(result.is_err());
    }
}
