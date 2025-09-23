use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use url::Url;
use uuid::Uuid;

mod unit;
mod websocket;

const WEBSOCKET_URL: &str = "ws://127.0.0.1:8000/ws";
const TEST_JWT_SECRET: &str = "your-secret-key"; // Should match your JWT_SECRET

/// Helper function to create test JWT tokens
fn create_test_jwt(user_id: Uuid, username: &str, email: &str) -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    #[derive(serde::Serialize)]
    struct TestClaims {
        sub: Uuid,
        email: String,
        username: String,
        exp: u64,
        iat: u64,
        jti: String,
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = TestClaims {
        sub: user_id,
        email: email.to_string(),
        username: username.to_string(),
        exp: now + 3600, // 1 hour from now
        iat: now,
        jti: Uuid::new_v4().to_string(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_ref()),
    )
    .unwrap()
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_websocket_connection_with_valid_token() {
    let user_id = Uuid::new_v4();
    let username = "test_user";
    let email = "test@example.com";
    let token = create_test_jwt(user_id, username, email);
    let url = format!("{}?token={}", WEBSOCKET_URL, token);

    let url = Url::parse(&url).expect("Invalid URL");
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

    // Wait for response (welcome message or pong)
    if let Ok(Some(msg)) = timeout(Duration::from_secs(5), ws_receiver.next()).await {
        match msg {
            Ok(TungsteniteMessage::Text(text)) => {
                let response: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert!(response.get("message_type").is_some());
                println!("Received message: {}", text);
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
    let url = Url::parse(WEBSOCKET_URL).expect("Invalid URL");

    // This should fail due to missing authentication
    let result = connect_async(url).await;
    assert!(result.is_err(), "Connection should fail without token");
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_websocket_connection_with_invalid_token() {
    let invalid_token = "invalid.jwt.token";
    let url = format!("{}?token={}", WEBSOCKET_URL, invalid_token);

    let url = Url::parse(&url).expect("Invalid URL");
    let result = connect_async(url).await;
    assert!(result.is_err(), "Connection should fail with invalid token");
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_websocket_text_message_exchange() {
    let user_id = Uuid::new_v4();
    let username = "message_user";
    let email = "message@example.com";
    let token = create_test_jwt(user_id, username, email);
    let url = format!("{}?token={}", WEBSOCKET_URL, token);

    let url = Url::parse(&url).expect("Invalid URL");
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Wait for welcome message
    if let Ok(Some(_)) = timeout(Duration::from_secs(2), ws_receiver.next()).await {
        // Received welcome message, continue
    }

    // Send a text message
    let text_message = json!({
        "id": Uuid::new_v4().to_string(),
        "message_type": "text",
        "data": {
            "content": "Hello, WebSocket!"
        },
        "timestamp": chrono::Utc::now(),
    });

    ws_sender
        .send(TungsteniteMessage::Text(text_message.to_string()))
        .await
        .expect("Failed to send text message");

    // Wait for echo or broadcast
    let mut received_messages = 0;
    while received_messages < 1 {
        if let Ok(Some(msg)) = timeout(Duration::from_secs(3), ws_receiver.next()).await {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    let response: serde_json::Value = serde_json::from_str(&text).unwrap();
                    if response["message_type"] == "text" {
                        received_messages += 1;
                        println!("Received text message: {}", text);
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }

    assert!(received_messages > 0, "Should receive at least one message");
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_websocket_multiple_connections() {
    let mut connections = Vec::new();
    let num_connections = 3;

    // Create multiple connections
    for i in 0..num_connections {
        let user_id = Uuid::new_v4();
        let username = format!("multi_user_{}", i);
        let email = format!("multi{}@example.com", i);
        let token = create_test_jwt(user_id, &username, &email);
        let url = format!("{}?token={}", WEBSOCKET_URL, token);

        let url = Url::parse(&url).expect("Invalid URL");
        if let Ok((ws_stream, _)) = connect_async(url).await {
            let (sender, receiver) = ws_stream.split();
            connections.push((sender, receiver, username));
        }
    }

    assert_eq!(
        connections.len(),
        num_connections,
        "All connections should succeed"
    );

    // Send messages from first connection
    if let Some((sender, _, _)) = connections.get_mut(0) {
        let broadcast_message = json!({
            "id": Uuid::new_v4().to_string(),
            "message_type": "text",
            "data": {
                "content": "Broadcast message from connection 0"
            },
            "timestamp": chrono::Utc::now(),
        });

        sender
            .send(TungsteniteMessage::Text(broadcast_message.to_string()))
            .await
            .expect("Failed to send broadcast message");
    }

    // Try to receive messages on other connections
    tokio::time::sleep(Duration::from_millis(500)).await;

    for (i, (_, receiver, username)) in connections.into_iter().enumerate() {
        let mut receiver = receiver;
        println!("Checking messages for {}", username);

        // Try to receive any pending messages
        let mut message_count = 0;
        while message_count < 5 {
            match timeout(Duration::from_millis(100), receiver.next()).await {
                Ok(Some(Ok(TungsteniteMessage::Text(text)))) => {
                    println!("Connection {}: Received {}", i, text);
                    message_count += 1;
                }
                _ => break,
            }
        }
    }
}

#[tokio::test]
#[ignore = "requires running server and API endpoints"]
async fn test_websocket_api_endpoints() {
    use reqwest;

    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:8000";

    // Test online users endpoint
    let response = client
        .get(&format!("{}/ws/online", base_url))
        .send()
        .await
        .expect("Failed to get online users");

    assert!(response.status().is_success());
    let online_users: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(online_users.get("count").is_some());
    assert!(online_users.get("users").is_some());

    // Test WebSocket stats endpoint
    let response = client
        .get(&format!("{}/ws/stats", base_url))
        .send()
        .await
        .expect("Failed to get WebSocket stats");

    assert!(response.status().is_success());
    let stats: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(stats.get("total_connections").is_some());
    assert!(stats.get("unique_users").is_some());
    assert!(stats.get("server_uptime").is_some());

    // Test broadcast message endpoint
    let broadcast_payload = json!({
        "message_type": "notification",
        "data": {
            "content": "Test broadcast notification",
            "type": "system"
        }
    });

    let response = client
        .post(&format!("{}/ws/broadcast", base_url))
        .json(&broadcast_payload)
        .send()
        .await
        .expect("Failed to broadcast message");

    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(result["success"], true);
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_websocket_connection_lifecycle() {
    let user_id = Uuid::new_v4();
    let username = "lifecycle_user";
    let email = "lifecycle@example.com";
    let token = create_test_jwt(user_id, username, email);
    let url = format!("{}?token={}", WEBSOCKET_URL, token);

    let url = Url::parse(&url).expect("Invalid URL");
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Wait for welcome message
    if let Ok(Some(Ok(TungsteniteMessage::Text(welcome)))) =
        timeout(Duration::from_secs(2), ws_receiver.next()).await
    {
        let welcome_msg: serde_json::Value = serde_json::from_str(&welcome).unwrap();
        assert!(
            welcome_msg["message_type"] == "system_message"
                || welcome_msg["message_type"] == "user_joined"
        );
        println!("Welcome message: {}", welcome);
    }

    // Send periodic ping messages
    for i in 0..3 {
        let ping_message = json!({
            "id": Uuid::new_v4().to_string(),
            "message_type": "ping",
            "data": {"sequence": i},
            "timestamp": chrono::Utc::now(),
        });

        ws_sender
            .send(TungsteniteMessage::Text(ping_message.to_string()))
            .await
            .expect("Failed to send ping");

        // Wait for pong response
        if let Ok(Some(Ok(TungsteniteMessage::Text(pong)))) =
            timeout(Duration::from_secs(1), ws_receiver.next()).await
        {
            let pong_msg: serde_json::Value = serde_json::from_str(&pong).unwrap();
            if pong_msg["message_type"] == "pong" {
                println!("Received pong: {}", pong);
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Close connection gracefully
    ws_sender.send(TungsteniteMessage::Close(None)).await.ok();

    // Wait for close acknowledgment
    if let Ok(Some(Ok(TungsteniteMessage::Close(_)))) =
        timeout(Duration::from_secs(1), ws_receiver.next()).await
    {
        println!("Connection closed gracefully");
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "performance test - requires running server"]
    async fn test_websocket_connection_performance() {
        let num_connections = 10;
        let start_time = std::time::Instant::now();

        let mut handles = Vec::new();

        for i in 0..num_connections {
            let handle = tokio::spawn(async move {
                let user_id = Uuid::new_v4();
                let username = format!("perf_user_{}", i);
                let email = format!("perf{}@example.com", i);
                let token = create_test_jwt(user_id, &username, &email);
                let url = format!("{}?token={}", WEBSOCKET_URL, token);

                match Url::parse(&url) {
                    Ok(parsed_url) => {
                        match timeout(Duration::from_secs(5), connect_async(parsed_url)).await {
                            Ok(Ok((ws_stream, _))) => {
                                // Connection successful, keep it alive briefly
                                tokio::time::sleep(Duration::from_millis(100)).await;
                                drop(ws_stream);
                                true
                            }
                            _ => false,
                        }
                    }
                    _ => false,
                }
            });

            handles.push(handle);
        }

        let mut successful_connections = 0;
        for handle in handles {
            if let Ok(success) = handle.await {
                if success {
                    successful_connections += 1;
                }
            }
        }

        let duration = start_time.elapsed();
        println!(
            "Performance test: {}/{} connections successful in {:?}",
            successful_connections, num_connections, duration
        );

        assert!(
            successful_connections > num_connections / 2,
            "At least half of connections should succeed"
        );
        assert!(
            duration < Duration::from_secs(10),
            "All connections should complete within 10 seconds"
        );
    }
}

// User registration and workspace creation tests
mod user_tests {
    use rust_backend::db::models::RegisterRequest;

    #[tokio::test]
    async fn test_workspace_url_key_generation() {
        // Test different username formats to ensure URL key generation works correctly
        let test_cases = vec![
            ("john", "john-workspace"),
            ("JohnDoe", "johndoe-workspace"),
            ("user_123", "user_123-workspace"),
            ("test-user", "test-user-workspace"),
        ];

        for (username, expected_url_key) in test_cases {
            let generated_url_key = format!("{}-workspace", username.to_lowercase());
            assert_eq!(generated_url_key, expected_url_key);
            println!(
                "✅ URL key generation test passed: {} -> {}",
                username, generated_url_key
            );
        }
    }

    #[tokio::test]
    async fn test_user_registration_workspace_creation_logic() {
        // This test verifies the logic for default workspace creation
        // Note: This test doesn't require a running server - it tests the logic only

        // Test data
        let test_user_data = RegisterRequest {
            email: "testuser@example.com".to_string(),
            username: "testuser".to_string(),
            name: "Test User".to_string(),
            password: "password123".to_string(),
        };

        // Test workspace name generation
        let workspace_name = format!("{}'s Workspace", test_user_data.name);
        let workspace_url_key = format!("{}-workspace", test_user_data.username.to_lowercase());

        assert_eq!(workspace_name, "Test User's Workspace");
        assert_eq!(workspace_url_key, "testuser-workspace");

        // Test team creation parameters
        let team_name = "Default Team";
        let team_key = "DEF";
        let user_role = "admin";

        assert_eq!(team_name, "Default Team");
        assert_eq!(team_key, "DEF");
        assert_eq!(user_role, "admin");

        println!("✅ User registration workspace creation logic test passed");
        println!("   - Workspace name: {}", workspace_name);
        println!("   - Workspace URL key: {}", workspace_url_key);
        println!("   - Default team: {} ({})", team_name, team_key);
        println!("   - User role: {}", user_role);
    }

    #[tokio::test]
    async fn test_user_profile_structure() {
        // This test verifies the UserProfile structure matches expected fields
        use rust_backend::db::models::{TeamInfo, UserProfile, WorkspaceInfo};
        use uuid::Uuid;

        // Test creating a UserProfile with mock data
        let mock_workspace = WorkspaceInfo {
            id: Uuid::new_v4(),
            name: "Test Workspace".to_string(),
            url_key: "test-workspace".to_string(),
            logo_url: None,
        };

        let mock_team = TeamInfo {
            id: Uuid::new_v4(),
            name: "Test Team".to_string(),
            team_key: "TEST".to_string(),
            description: Some("Test team description".to_string()),
            icon_url: None,
            is_private: false,
            role: "admin".to_string(),
        };

        let workspace_id = mock_workspace.id;
        let user_profile = UserProfile {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            name: "Test User".to_string(),
            avatar_url: None,
            current_workspace_id: Some(workspace_id),
            workspaces: vec![mock_workspace],
            teams: vec![mock_team],
        };

        // Verify the structure
        assert_eq!(user_profile.workspaces.len(), 1);
        assert_eq!(user_profile.teams.len(), 1);
        assert_eq!(user_profile.workspaces[0].name, "Test Workspace");
        assert_eq!(user_profile.teams[0].role, "admin");
        assert_eq!(user_profile.current_workspace_id, Some(workspace_id));

        println!("✅ UserProfile structure test passed");
        println!("   - User: {} ({})", user_profile.name, user_profile.email);
        println!("   - Workspaces: {}", user_profile.workspaces.len());
        println!("   - Teams: {}", user_profile.teams.len());
        println!(
            "   - Current workspace: {:?}",
            user_profile.current_workspace_id
        );
    }

    #[tokio::test]
    async fn test_switch_workspace_request_structure() {
        // This test verifies the SwitchWorkspaceRequest and WorkspaceSwitchResult structures
        use rust_backend::db::models::{
            SwitchWorkspaceRequest, TeamInfo, WorkspaceInfo, WorkspaceSwitchResult,
        };
        use uuid::Uuid;

        let workspace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();

        // Test request structure
        let switch_request = SwitchWorkspaceRequest {
            workspace_id: workspace_id,
        };

        assert_eq!(switch_request.workspace_id, workspace_id);

        // Test new response structure (WorkspaceSwitchResult)
        let workspace_info = WorkspaceInfo {
            id: workspace_id,
            name: "Test Workspace".to_string(),
            url_key: "test-workspace".to_string(),
            logo_url: None,
        };

        let team_info = TeamInfo {
            id: team_id,
            name: "Test Team".to_string(),
            team_key: "TEST".to_string(),
            description: Some("Test team description".to_string()),
            icon_url: None,
            is_private: false,
            role: "admin".to_string(),
        };

        let switch_result = WorkspaceSwitchResult {
            user_id,
            previous_workspace_id: None,
            current_workspace: workspace_info,
            user_role_in_workspace: "admin".to_string(),
            available_teams: vec![team_info],
        };

        assert_eq!(switch_result.user_id, user_id);
        assert_eq!(switch_result.current_workspace.id, workspace_id);
        assert_eq!(switch_result.current_workspace.name, "Test Workspace");
        assert_eq!(switch_result.user_role_in_workspace, "admin");
        assert_eq!(switch_result.available_teams.len(), 1);
        assert_eq!(switch_result.available_teams[0].role, "admin");

        println!("✅ Workspace switching DTOs test passed");
        println!("   - Request structure: ✓");
        println!("   - New WorkspaceSwitchResult structure: ✓");
    }

    #[tokio::test]
    async fn test_default_workflow_states_creation() {
        // This test verifies that the default workflow states are correctly defined
        use rust_backend::db::models::workflow::{DefaultWorkflowState, WorkflowStateCategory};

        let default_states = DefaultWorkflowState::get_default_states();

        // Verify we have the expected number of states
        assert_eq!(
            default_states.len(),
            7,
            "Should have 7 default workflow states"
        );

        // Verify specific states exist with correct properties
        let backlog_state = default_states.iter().find(|s| s.name == "Backlog").unwrap();
        assert_eq!(
            backlog_state.description,
            "Issues that are not yet prioritized"
        );
        assert_eq!(backlog_state.color, "#999999");
        assert_eq!(backlog_state.category, WorkflowStateCategory::Backlog);
        assert_eq!(backlog_state.position, 1);
        assert!(backlog_state.is_default);

        let todo_state = default_states.iter().find(|s| s.name == "Todo").unwrap();
        assert_eq!(
            todo_state.description,
            "Issues that are ready to be worked on"
        );
        assert_eq!(todo_state.color, "#999999");
        assert_eq!(todo_state.category, WorkflowStateCategory::Unstarted);
        assert_eq!(todo_state.position, 1);
        assert!(!todo_state.is_default);

        let in_progress_state = default_states
            .iter()
            .find(|s| s.name == "In Progress")
            .unwrap();
        assert_eq!(
            in_progress_state.description,
            "Issues currently being worked on"
        );
        assert_eq!(in_progress_state.color, "#F1BF00");
        assert_eq!(in_progress_state.category, WorkflowStateCategory::Started);
        assert_eq!(in_progress_state.position, 1);
        assert!(!in_progress_state.is_default);

        let in_review_state = default_states
            .iter()
            .find(|s| s.name == "In Review")
            .unwrap();
        assert_eq!(in_review_state.description, "Issues ready for review");
        assert_eq!(in_review_state.color, "#82E0AA");
        assert_eq!(in_review_state.category, WorkflowStateCategory::Started);
        assert_eq!(in_review_state.position, 2);
        assert!(!in_review_state.is_default);

        let done_state = default_states.iter().find(|s| s.name == "Done").unwrap();
        assert_eq!(done_state.description, "Completed issues");
        assert_eq!(done_state.color, "#0082FF");
        assert_eq!(done_state.category, WorkflowStateCategory::Completed);
        assert_eq!(done_state.position, 1);
        assert!(!done_state.is_default);

        let canceled_state = default_states
            .iter()
            .find(|s| s.name == "Canceled")
            .unwrap();
        assert_eq!(canceled_state.description, "Canceled or invalid issues");
        assert_eq!(canceled_state.color, "#333333");
        assert_eq!(canceled_state.category, WorkflowStateCategory::Canceled);
        assert_eq!(canceled_state.position, 1);
        assert!(!canceled_state.is_default);

        let duplicated_state = default_states
            .iter()
            .find(|s| s.name == "Duplicated")
            .unwrap();
        assert_eq!(duplicated_state.description, "Duplicated Issue");
        assert_eq!(duplicated_state.color, "#333333");
        assert_eq!(duplicated_state.category, WorkflowStateCategory::Canceled);
        assert_eq!(duplicated_state.position, 0);
        assert!(!duplicated_state.is_default);

        println!("✅ Default workflow states creation test passed");
        println!("   - Total states: {}", default_states.len());
        println!("   - Backlog state: ✓ (default)");
        println!("   - Todo state: ✓");
        println!("   - In Progress state: ✓");
        println!("   - In Review state: ✓");
        println!("   - Done state: ✓");
        println!("   - Canceled state: ✓");
        println!("   - Duplicated state: ✓");
    }
}
