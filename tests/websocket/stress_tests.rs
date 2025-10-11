use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Barrier, Semaphore};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use url::Url;
use uuid::Uuid;

use rust_backend::websocket::manager::{MessageType, WebSocketManager, WebSocketMessage};

const WEBSOCKET_URL: &str = "ws://127.0.0.1:8000/ws";
const TEST_JWT_SECRET: &str = "test_jwt_secret_key";

// Helper function to create test JWT tokens
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
async fn stress_test_websocket_manager_concurrent_connections() {
    let manager = WebSocketManager::new();
    let num_connections = 1000;
    let barrier = Arc::new(Barrier::new(num_connections));
    let success_count = Arc::new(AtomicUsize::new(0));

    let start_time = Instant::now();

    let mut handles = Vec::new();

    for i in 0..num_connections {
        let manager_clone = manager.clone();
        let barrier_clone = Arc::clone(&barrier);
        let success_count_clone = Arc::clone(&success_count);

        let handle = tokio::spawn(async move {
            let user_id = Uuid::new_v4();
            let connection_id = format!("stress_connection_{}", i);

            let user = rust_backend::websocket::manager::ConnectedUser {
                user_id,
                username: format!("stress_user_{}", i),
                connected_at: chrono::Utc::now(),
                last_ping: chrono::Utc::now(),
                state: rust_backend::websocket::manager::ConnectionState::Connected,
                subscriptions: std::collections::HashSet::new(),
                message_queue: std::collections::VecDeque::new(),
                recovery_token: None,
                metadata: std::collections::HashMap::new(),
                current_workspace_id: Some(Uuid::new_v4()),
            };

            // Wait for all tasks to be ready
            barrier_clone.wait().await;

            // Add connection
            manager_clone
                .add_connection(connection_id.clone(), user, None, None)
                .await;
            success_count_clone.fetch_add(1, Ordering::Relaxed);

            // Keep connection alive for a short time
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Remove connection
            manager_clone.remove_connection(&connection_id).await;
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start_time.elapsed();
    let final_count = manager.get_connection_count().await;

    println!(
        "Concurrent connections test: {} connections in {:?}, final count: {}",
        success_count.load(Ordering::Relaxed),
        duration,
        final_count
    );

    // All connections should have been added and removed
    assert_eq!(success_count.load(Ordering::Relaxed), num_connections);
    assert_eq!(final_count, 0);

    // Should complete within reasonable time (10 seconds for 1000 connections)
    assert!(duration < Duration::from_secs(10));
}

#[tokio::test]
async fn stress_test_websocket_manager_message_broadcasting() {
    let manager = WebSocketManager::new();
    let num_connections = 100;
    let messages_per_connection = 10;

    // Add connections
    for i in 0..num_connections {
        let user_id = Uuid::new_v4();
        let connection_id = format!("broadcast_connection_{}", i);

        let user = rust_backend::websocket::manager::ConnectedUser {
            user_id,
            username: format!("broadcast_user_{}", i),
            connected_at: chrono::Utc::now(),
            last_ping: chrono::Utc::now(),
            state: rust_backend::websocket::manager::ConnectionState::Connected,
            subscriptions: std::collections::HashSet::new(),
            message_queue: std::collections::VecDeque::new(),
            recovery_token: None,
            metadata: std::collections::HashMap::new(),
            current_workspace_id: Some(Uuid::new_v4()),
        };

        manager
            .add_connection(connection_id, user, None, None)
            .await;
    }

    assert_eq!(manager.get_connection_count().await, num_connections);

    let start_time = Instant::now();
    let broadcast_count = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    // Create multiple broadcasters
    for i in 0..messages_per_connection {
        let manager_clone = manager.clone();
        let broadcast_count_clone = Arc::clone(&broadcast_count);

        let handle = tokio::spawn(async move {
            let message = WebSocketMessage {
                id: Some(Uuid::new_v4().to_string()),
                message_type: MessageType::Text,
                data: json!({
                    "content": format!("Stress test message {}", i),
                    "timestamp": chrono::Utc::now()
                }),
                timestamp: Some(chrono::Utc::now()),
            };

            manager_clone.broadcast_message(message).await;
            broadcast_count_clone.fetch_add(1, Ordering::Relaxed);
        });

        handles.push(handle);
    }

    // Wait for all broadcasts to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start_time.elapsed();

    println!(
        "Message broadcasting test: {} messages to {} connections in {:?}",
        broadcast_count.load(Ordering::Relaxed),
        num_connections,
        duration
    );

    assert_eq!(
        broadcast_count.load(Ordering::Relaxed),
        messages_per_connection
    );

    // Should complete within reasonable time
    assert!(duration < Duration::from_secs(5));

    // Cleanup
    for i in 0..num_connections {
        let connection_id = format!("broadcast_connection_{}", i);
        manager.remove_connection(&connection_id).await;
    }

    assert_eq!(manager.get_connection_count().await, 0);
}

#[tokio::test]
async fn stress_test_websocket_manager_rapid_connect_disconnect() {
    let manager = WebSocketManager::new();
    let num_cycles = 500;
    let connections_per_cycle = 10;

    let start_time = Instant::now();

    for cycle in 0..num_cycles {
        let mut cycle_handles = Vec::new();

        // Rapid connect
        for i in 0..connections_per_cycle {
            let manager_clone = manager.clone();
            let connection_id = format!("rapid_connection_{}_{}", cycle, i);

            let handle = tokio::spawn(async move {
                let user_id = Uuid::new_v4();
                let user = rust_backend::websocket::manager::ConnectedUser {
                    user_id,
                    username: format!("rapid_user_{}_{}", cycle, i),
                    connected_at: chrono::Utc::now(),
                    last_ping: chrono::Utc::now(),
                    state: rust_backend::websocket::manager::ConnectionState::Connected,
                    subscriptions: std::collections::HashSet::new(),
                    message_queue: std::collections::VecDeque::new(),
                    recovery_token: None,
                    metadata: std::collections::HashMap::new(),
                    current_workspace_id: Some(Uuid::new_v4()),
                };

                manager_clone
                    .add_connection(connection_id.clone(), user, None, None)
                    .await;

                // Very short connection lifetime
                tokio::time::sleep(Duration::from_millis(1)).await;

                manager_clone.remove_connection(&connection_id).await;
            });

            cycle_handles.push(handle);
        }

        // Wait for cycle to complete
        for handle in cycle_handles {
            handle.await.unwrap();
        }

        // Verify no connections remain
        if cycle % 100 == 0 {
            let count = manager.get_connection_count().await;
            assert_eq!(
                count, 0,
                "Cycle {}: {} connections still active",
                cycle, count
            );
        }
    }

    let duration = start_time.elapsed();
    let final_count = manager.get_connection_count().await;

    println!(
        "Rapid connect/disconnect test: {} cycles of {} connections in {:?}",
        num_cycles, connections_per_cycle, duration
    );

    assert_eq!(final_count, 0);
    assert!(duration < Duration::from_secs(30));
}

#[tokio::test]
async fn stress_test_websocket_manager_memory_usage() {
    let manager = WebSocketManager::new();
    let num_connections = 2000;
    let large_data_size = 1024 * 10; // 10KB per message

    // Add connections with large usernames
    for i in 0..num_connections {
        let user_id = Uuid::new_v4();
        let connection_id = format!("memory_connection_{}", i);
        let large_username = "x".repeat(large_data_size);

        let user = rust_backend::websocket::manager::ConnectedUser {
            user_id,
            username: large_username,
            connected_at: chrono::Utc::now(),
            last_ping: chrono::Utc::now(),
            state: rust_backend::websocket::manager::ConnectionState::Connected,
            subscriptions: std::collections::HashSet::new(),
            message_queue: std::collections::VecDeque::new(),
            recovery_token: None,
            metadata: std::collections::HashMap::new(),
            current_workspace_id: Some(Uuid::new_v4()),
        };

        manager
            .add_connection(connection_id, user, None, None)
            .await;

        // Periodic memory pressure test
        if i % 500 == 0 {
            let count = manager.get_connection_count().await;
            assert_eq!(count, i + 1);
        }
    }

    assert_eq!(manager.get_connection_count().await, num_connections);

    // Test large message broadcasting
    let large_message = WebSocketMessage {
        id: Some(Uuid::new_v4().to_string()),
        message_type: MessageType::Text,
        data: json!({
            "content": "x".repeat(large_data_size),
            "large_array": vec![1; 1000]
        }),
        timestamp: Some(chrono::Utc::now()),
    };

    let broadcast_start = Instant::now();
    manager.broadcast_message(large_message).await;
    let broadcast_duration = broadcast_start.elapsed();

    println!(
        "Memory stress test: {} connections with large data, broadcast took {:?}",
        num_connections, broadcast_duration
    );

    // Cleanup
    let cleanup_start = Instant::now();
    for i in 0..num_connections {
        let connection_id = format!("memory_connection_{}", i);
        manager.remove_connection(&connection_id).await;
    }
    let cleanup_duration = cleanup_start.elapsed();

    println!("Cleanup took {:?}", cleanup_duration);

    assert_eq!(manager.get_connection_count().await, 0);
    assert!(broadcast_duration < Duration::from_secs(10));
    assert!(cleanup_duration < Duration::from_secs(10));
}

#[tokio::test]
async fn stress_test_websocket_manager_ping_updates() {
    let manager = WebSocketManager::new();
    let num_connections = 100;
    let pings_per_connection = 50;

    // Add connections
    for i in 0..num_connections {
        let user_id = Uuid::new_v4();
        let connection_id = format!("ping_connection_{}", i);

        let user = rust_backend::websocket::manager::ConnectedUser {
            user_id,
            username: format!("ping_user_{}", i),
            connected_at: chrono::Utc::now(),
            last_ping: chrono::Utc::now(),
            state: rust_backend::websocket::manager::ConnectionState::Connected,
            subscriptions: std::collections::HashSet::new(),
            message_queue: std::collections::VecDeque::new(),
            recovery_token: None,
            metadata: std::collections::HashMap::new(),
            current_workspace_id: Some(Uuid::new_v4()),
        };

        manager
            .add_connection(connection_id, user, None, None)
            .await;
    }

    let start_time = Instant::now();
    let total_pings = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    // Create ping stress test
    for i in 0..num_connections {
        let manager_clone = manager.clone();
        let total_pings_clone = Arc::clone(&total_pings);
        let connection_id = format!("ping_connection_{}", i);

        let handle = tokio::spawn(async move {
            for _ in 0..pings_per_connection {
                manager_clone.update_ping(&connection_id).await;
                total_pings_clone.fetch_add(1, Ordering::Relaxed);

                // Small delay to simulate realistic ping intervals
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });

        handles.push(handle);
    }

    // Wait for all ping updates
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start_time.elapsed();
    let expected_pings = num_connections * pings_per_connection;

    println!(
        "Ping update stress test: {} ping updates in {:?}",
        total_pings.load(Ordering::Relaxed),
        duration
    );

    assert_eq!(total_pings.load(Ordering::Relaxed), expected_pings);
    assert!(duration < Duration::from_secs(10));

    // Cleanup
    for i in 0..num_connections {
        let connection_id = format!("ping_connection_{}", i);
        manager.remove_connection(&connection_id).await;
    }

    assert_eq!(manager.get_connection_count().await, 0);
}

#[tokio::test]
async fn stress_test_websocket_manager_concurrent_operations() {
    let manager = WebSocketManager::new();
    let num_operations = 1000;
    let semaphore = Arc::new(Semaphore::new(50)); // Limit concurrent operations

    let start_time = Instant::now();
    let mut handles = Vec::new();

    for i in 0..num_operations {
        let manager_clone = manager.clone();
        let semaphore_clone = Arc::clone(&semaphore);

        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();

            let user_id = Uuid::new_v4();
            let connection_id = format!("concurrent_connection_{}", i);

            // Random operation type
            match i % 4 {
                0 => {
                    // Add connection
                    let user = rust_backend::websocket::manager::ConnectedUser {
                        user_id,
                        username: format!("concurrent_user_{}", i),
                        connected_at: chrono::Utc::now(),
                        last_ping: chrono::Utc::now(),
                        state: rust_backend::websocket::manager::ConnectionState::Connected,
                        subscriptions: std::collections::HashSet::new(),
                        message_queue: std::collections::VecDeque::new(),
                        recovery_token: None,
                        metadata: std::collections::HashMap::new(),
                        current_workspace_id: Some(Uuid::new_v4()),
                    };
                    manager_clone
                        .add_connection(connection_id, user, None, None)
                        .await;
                }
                1 => {
                    // Get connection count
                    let _count = manager_clone.get_connection_count().await;
                }
                2 => {
                    // Get online users
                    let _users = manager_clone.get_online_users().await;
                }
                3 => {
                    // Broadcast message
                    let message = WebSocketMessage {
                        id: Some(Uuid::new_v4().to_string()),
                        message_type: MessageType::Text,
                        data: json!({"content": format!("Concurrent message {}", i)}),
                        timestamp: Some(chrono::Utc::now()),
                    };
                    manager_clone.broadcast_message(message).await;
                }
                _ => unreachable!(),
            }
        });

        handles.push(handle);
    }

    // Wait for all operations
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start_time.elapsed();
    let final_count = manager.get_connection_count().await;

    println!(
        "Concurrent operations test: {} operations in {:?}, final connections: {}",
        num_operations, duration, final_count
    );

    assert!(duration < Duration::from_secs(30));

    // Cleanup remaining connections
    manager.cleanup_stale_connections(0).await;
}

#[tokio::test]
async fn stress_test_websocket_manager_subscription_performance() {
    let manager = WebSocketManager::new();
    let num_subscribers = 100;
    let num_messages = 50;

    let start_time = Instant::now();
    let received_count = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    // Create multiple subscribers
    for _i in 0..num_subscribers {
        let mut rx = manager.get_broadcast_receiver();
        let received_count_clone = Arc::clone(&received_count);

        let handle = tokio::spawn(async move {
            let mut local_received = 0;

            while local_received < num_messages {
                match timeout(Duration::from_secs(5), rx.recv()).await {
                    Ok(Ok(_message)) => {
                        local_received += 1;
                        received_count_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(Err(_)) => break, // Channel closed
                    Err(_) => break,     // Timeout
                }
            }
        });

        handles.push(handle);
    }

    // Give subscribers time to set up
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Broadcast messages
    for i in 0..num_messages {
        let message = WebSocketMessage {
            id: Some(Uuid::new_v4().to_string()),
            message_type: MessageType::Text,
            data: json!({
                "content": format!("Subscription test message {}", i),
                "sequence": i
            }),
            timestamp: Some(chrono::Utc::now()),
        };

        manager.broadcast_message(message).await;
        tokio::time::sleep(Duration::from_millis(10)).await; // Small delay
    }

    // Wait for all subscribers to finish
    for handle in handles {
        let _ = handle.await;
    }

    let duration = start_time.elapsed();
    let total_received = received_count.load(Ordering::Relaxed);
    let expected_total = num_subscribers * num_messages;

    println!(
        "Subscription performance test: {}/{} messages received by {} subscribers in {:?}",
        total_received, expected_total, num_subscribers, duration
    );

    // We might not receive all messages due to timing, but should receive most
    assert!(total_received > expected_total * 80 / 100); // At least 80%
    assert!(duration < Duration::from_secs(10));
}

// Integration stress tests (require running server)
#[cfg(test)]
mod integration_stress_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn stress_test_real_websocket_connections() {
        let num_connections = 50; // Keep reasonable for integration tests
        let semaphore = Arc::new(Semaphore::new(10)); // Limit concurrent connections

        let start_time = Instant::now();
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for i in 0..num_connections {
            let semaphore_clone = Arc::clone(&semaphore);
            let success_count_clone = Arc::clone(&success_count);

            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();

                let user_id = Uuid::new_v4();
                let username = format!("stress_user_{}", i);
                let token = create_test_jwt(user_id, &username, TEST_JWT_SECRET);
                let url = format!("{}?token={}", WEBSOCKET_URL, token);

                match Url::parse(&url) {
                    Ok(parsed_url) => {
                        match timeout(Duration::from_secs(5), connect_async(parsed_url)).await {
                            Ok(Ok((ws_stream, _))) => {
                                let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                                // Send a few messages
                                for msg_num in 0..5 {
                                    let message = json!({
                                        "id": Uuid::new_v4().to_string(),
                                        "message_type": "text",
                                        "data": {
                                            "content": format!("Stress message {} from user {}", msg_num, i)
                                        },
                                        "timestamp": chrono::Utc::now(),
                                    });

                                    if ws_sender
                                        .send(TungsteniteMessage::Text(message.to_string()))
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }

                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                }

                                // Try to receive at least one message
                                if let Ok(Some(_)) =
                                    timeout(Duration::from_secs(1), ws_receiver.next()).await
                                {
                                    success_count_clone.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            _ => {
                                // Connection failed, but that's expected under high load
                            }
                        }
                    }
                    Err(_) => {
                        // URL parsing failed
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all connection attempts
        for handle in handles {
            let _ = handle.await;
        }

        let duration = start_time.elapsed();
        let successful_connections = success_count.load(Ordering::Relaxed);

        println!(
            "Real WebSocket stress test: {}/{} successful connections in {:?}",
            successful_connections, num_connections, duration
        );

        // We expect at least some connections to succeed
        assert!(successful_connections > 0);
        assert!(duration < Duration::from_secs(30));
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn stress_test_websocket_message_throughput() {
        let user_id = Uuid::new_v4();
        let token = create_test_jwt(user_id, "throughput_user", TEST_JWT_SECRET);
        let url = format!("{}?token={}", WEBSOCKET_URL, token);

        let url = Url::parse(&url).unwrap();
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        let num_messages = 1000;
        let start_time = Instant::now();
        let received_count = Arc::new(AtomicUsize::new(0));

        // Spawn receiver task
        let received_count_clone = Arc::clone(&received_count);
        let receive_handle = tokio::spawn(async move {
            while received_count_clone.load(Ordering::Relaxed) < num_messages {
                match timeout(Duration::from_secs(1), ws_receiver.next()).await {
                    Ok(Some(Ok(_))) => {
                        received_count_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    _ => break,
                }
            }
        });

        // Send messages rapidly
        let send_start = Instant::now();
        for i in 0..num_messages {
            let message = json!({
                "id": Uuid::new_v4().to_string(),
                "message_type": "text",
                "data": {
                    "content": format!("Throughput test message {}", i),
                    "sequence": i
                },
                "timestamp": chrono::Utc::now(),
            });

            if ws_sender
                .send(TungsteniteMessage::Text(message.to_string()))
                .await
                .is_err()
            {
                break;
            }

            // Small delay to avoid overwhelming the server
            if i % 100 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }

        let send_duration = send_start.elapsed();

        // Wait a bit more for all messages to be processed
        tokio::time::sleep(Duration::from_secs(2)).await;
        receive_handle.abort();

        let total_duration = start_time.elapsed();
        let received = received_count.load(Ordering::Relaxed);

        println!(
            "Message throughput test: sent {} messages in {:?}, received {} in {:?}",
            num_messages, send_duration, received, total_duration
        );

        // We expect most messages to be echoed back
        assert!(received > num_messages * 50 / 100); // At least 50%
        assert!(send_duration < Duration::from_secs(30));
    }
}
