//! Test Fixtures for WebSocket Integration Tests
//!
//! This module provides a complete test fixture system for integration testing.
//! These tests require a running server and database - run with: cargo test -- --ignored

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use url::Url;
use uuid::Uuid;

const WEBSOCKET_URL: &str = "ws://127.0.0.1:8000/ws";
const TEST_JWT_SECRET: &str = "test-secret-key";

/// Create test JWT tokens for authentication
pub fn create_test_jwt(user_id: Uuid, username: &str, email: &str, workspace_id: Uuid) -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    #[derive(serde::Serialize)]
    struct TestClaims {
        sub: Uuid,
        email: String,
        username: String,
        exp: u64,
        iat: u64,
        jti: String,
        workspace_id: Uuid,
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = TestClaims {
        sub: user_id,
        email: email.to_string(),
        username: username.to_string(),
        exp: now + 3600,
        iat: now,
        jti: Uuid::new_v4().to_string(),
        workspace_id,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_ref()),
    )
    .unwrap()
}

/// WebSocket connection handle for tests
pub struct WebSocketTestConnection {
    pub sender: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        TungsteniteMessage,
    >,
    pub receiver: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    >,
}

impl WebSocketTestConnection {
    /// Send a command and get the response
    pub async fn send_command(&mut self, command: serde_json::Value) -> serde_json::Value {
        self.sender
            .send(TungsteniteMessage::Text(command.to_string()))
            .await
            .expect("Failed to send command");

        if let Ok(Some(msg)) = timeout(Duration::from_secs(5), self.receiver.next()).await {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    serde_json::from_str(&text).expect("Failed to parse response")
                }
                Ok(TungsteniteMessage::Close(_)) => {
                    panic!("Connection closed by server")
                }
                _ => panic!("Unexpected message type"),
            }
        } else {
            panic!("No response received within timeout");
        }
    }

    /// Send a command without waiting for response
    pub async fn send_command_no_wait(&mut self, command: serde_json::Value) {
        self.sender
            .send(TungsteniteMessage::Text(command.to_string()))
            .await
            .expect("Failed to send command");
    }

    /// Close the connection
    pub async fn close(self) {
        let _ = self.sender.close().await;
    }
}

/// Connect to WebSocket with a user
pub async fn connect_websocket(
    user_id: Uuid,
    username: &str,
    email: &str,
    workspace_id: Uuid,
) -> Result<WebSocketTestConnection, Box<dyn std::error::Error + Send + Sync>> {
    let token = create_test_jwt(user_id, username, email, workspace_id);
    let url = format!("{}?token={}", WEBSOCKET_URL, token);

    let (ws_stream, _) = connect_async(Url::parse(&url)?).await?;
    let (sender, receiver) = ws_stream.split();

    Ok(WebSocketTestConnection { sender, receiver })
}

/// Complete test fixture for WebSocket integration tests
///
/// This fixture sets up:
/// - Test workspace
/// - Test team
/// - Test users (member and admin)
/// - WebSocket connections for both users
///
/// Note: The actual data creation requires a running database.
/// This fixture provides the structure and helper methods.
pub struct TestFixture {
    pub workspace_id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub admin_user_id: Uuid,
    pub issue_id: Option<Uuid>,
    pub status_id: Option<Uuid>,
    pub member_ws: Option<WebSocketTestConnection>,
    pub admin_ws: Option<WebSocketTestConnection>,
}

impl TestFixture {
    /// Create a new test fixture with generated IDs
    pub fn new() -> Self {
        Self {
            workspace_id: Uuid::new_v4(),
            team_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            admin_user_id: Uuid::new_v4(),
            issue_id: None,
            status_id: None,
            member_ws: None,
            admin_ws: None,
        }
    }

    /// Connect both member and admin WebSocket connections
    pub async fn connect_all(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Connect as member
        let member_conn = connect_websocket(
            self.user_id,
            "test_user",
            "test@example.com",
            self.workspace_id,
        )
        .await?;
        self.member_ws = Some(member_conn);

        // Connect as admin
        let admin_conn = connect_websocket(
            self.admin_user_id,
            "admin_user",
            "admin@example.com",
            self.workspace_id,
        )
        .await?;
        self.admin_ws = Some(admin_conn);

        Ok(())
    }

    /// Connect only member WebSocket
    pub async fn connect_member(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let member_conn = connect_websocket(
            self.user_id,
            "test_user",
            "test@example.com",
            self.workspace_id,
        )
        .await?;
        self.member_ws = Some(member_conn);
        Ok(())
    }

    /// Connect only admin WebSocket
    pub async fn connect_admin(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let admin_conn = connect_websocket(
            self.admin_user_id,
            "admin_user",
            "admin@example.com",
            self.workspace_id,
        )
        .await?;
        self.admin_ws = Some(admin_conn);
        Ok(())
    }

    /// Clean up connections
    pub async fn cleanup(&mut self) {
        if let Some(conn) = self.member_ws.take() {
            conn.close().await;
        }
        if let Some(conn) = self.admin_ws.take() {
            conn.close().await;
        }
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        // Note: We can't do async cleanup in Drop, so cleanup() must be called explicitly
        // This is a limitation of Rust's Drop trait
    }
}

// ========================================================================
// Helper Functions for Building Test Commands
// ========================================================================

/// Create a base command structure with meta
pub fn base_command(
    command_type: &str,
    workspace_id: Uuid,
    user_id: Uuid,
    request_id: Option<String>,
) -> serde_json::Value {
    json!({
        "type": command_type,
        "request_id": request_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        "meta": {
            "workspaceId": workspace_id.to_string(),
            "userId": user_id.to_string(),
            "source": "integration_test"
        }
    })
}

/// Create a create_team_workflow_status command
pub fn create_team_workflow_status_command(
    team_id: Uuid,
    name: &str,
    category: &str,
    color: &str,
    position: i32,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("create_team_workflow_status", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd["data"] = json!({
        "name": name,
        "category": category,
        "color": color,
        "position": position
    });
    cmd
}

/// Create a get_team_workflow_statuses command
pub fn get_team_workflow_statuses_command(
    team_id: Uuid,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("get_team_workflow_statuses", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd
}

/// Create an update_team_workflow_status command
pub fn update_team_workflow_status_command(
    team_id: Uuid,
    status_id: Uuid,
    data: serde_json::Value,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("update_team_workflow_status", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd["status_id"] = serde_json::Value::String(status_id.to_string());
    cmd["data"] = data;
    cmd
}

/// Create a delete_team_workflow_status command
pub fn delete_team_workflow_status_command(
    team_id: Uuid,
    status_id: Uuid,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("delete_team_workflow_status", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd["status_id"] = serde_json::Value::String(status_id.to_string());
    cmd
}

/// Create a create_comment command
pub fn create_comment_command(
    issue_id: Uuid,
    content: &str,
    content_type: Option<&str>,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("create_comment", workspace_id, user_id, None);
    cmd["issue_id"] = serde_json::Value::String(issue_id.to_string());
    cmd["data"] = json!({
        "content": content,
        "content_type": content_type.unwrap_or("markdown")
    });
    cmd
}

/// Create a query_comments command
pub fn query_comments_command(
    issue_id: Uuid,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("query_comments", workspace_id, user_id, None);
    cmd["issue_id"] = serde_json::Value::String(issue_id.to_string());
    cmd
}

/// Create an update_comment command
pub fn update_comment_command(
    issue_id: Uuid,
    comment_id: Uuid,
    content: &str,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("update_comment", workspace_id, user_id, None);
    cmd["issue_id"] = serde_json::Value::String(issue_id.to_string());
    cmd["comment_id"] = serde_json::Value::String(comment_id.to_string());
    cmd["data"] = json!({
        "content": content
    });
    cmd
}

/// Create a delete_comment command
pub fn delete_comment_command(
    issue_id: Uuid,
    comment_id: Uuid,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("delete_comment", workspace_id, user_id, None);
    cmd["issue_id"] = serde_json::Value::String(issue_id.to_string());
    cmd["comment_id"] = serde_json::Value::String(comment_id.to_string());
    cmd
}

/// Create a get_workspace command
pub fn get_workspace_command(workspace_id: Uuid, user_id: Uuid) -> serde_json::Value {
    let mut cmd = base_command("get_workspace", workspace_id, user_id, None);
    cmd["workspace_id"] = serde_json::Value::String(workspace_id.to_string());
    cmd
}

/// Create a get_team command
pub fn get_team_command(team_id: Uuid, workspace_id: Uuid, user_id: Uuid) -> serde_json::Value {
    let mut cmd = base_command("get_team", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd
}

/// Create a create_team command
pub fn create_team_command(
    name: &str,
    team_key: &str,
    description: Option<&str>,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("create_team", workspace_id, user_id, None);
    cmd["data"] = json!({
        "name": name,
        "team_key": team_key,
        "description": description,
        "is_private": false
    });
    cmd
}

/// Create an update_team command
pub fn update_team_command(
    team_id: Uuid,
    data: serde_json::Value,
    workspace_id: Uuid,
    user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("update_team", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd["data"] = data;
    cmd
}

/// Create a delete_team command
pub fn delete_team_command(team_id: Uuid, workspace_id: Uuid, user_id: Uuid) -> serde_json::Value {
    let mut cmd = base_command("delete_team", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd
}

/// Create a get_workspace_member command
pub fn get_workspace_member_command(
    user_id: Uuid,
    workspace_id: Uuid,
    requesting_user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("get_workspace_member", workspace_id, requesting_user_id, None);
    cmd["user_id"] = serde_json::Value::String(user_id.to_string());
    cmd
}

/// Create an update_workspace_member command
pub fn update_workspace_member_command(
    target_user_id: Uuid,
    role: &str,
    workspace_id: Uuid,
    requesting_user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("update_workspace_member", workspace_id, requesting_user_id, None);
    cmd["user_id"] = serde_json::Value::String(target_user_id.to_string());
    cmd["data"] = json!({
        "role": role
    });
    cmd
}

/// Create a delete_workspace_member command
pub fn delete_workspace_member_command(
    target_user_id: Uuid,
    workspace_id: Uuid,
    requesting_user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("delete_workspace_member", workspace_id, requesting_user_id, None);
    cmd["user_id"] = serde_json::Value::String(target_user_id.to_string());
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_jwt() {
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let token = create_test_jwt(user_id, "test", "test@example.com", workspace_id);

        // JWT should have 3 parts
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_base_command_structure() {
        let workspace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let cmd = base_command("test_command", workspace_id, user_id, Some("req-123".to_string()));

        assert_eq!(cmd["type"], "test_command");
        assert_eq!(cmd["request_id"], "req-123");
        assert_eq!(cmd["meta"]["workspaceId"], workspace_id.to_string());
        assert_eq!(cmd["meta"]["userId"], user_id.to_string());
    }

    #[test]
    fn test_create_team_workflow_status_command() {
        let team_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let cmd = create_team_workflow_status_command(
            team_id,
            "In Progress",
            "started",
            "#FF0000",
            0,
            workspace_id,
            user_id,
        );

        assert_eq!(cmd["type"], "create_team_workflow_status");
        assert_eq!(cmd["team_id"], team_id.to_string());
        assert_eq!(cmd["data"]["name"], "In Progress");
        assert_eq!(cmd["data"]["category"], "started");
        assert_eq!(cmd["data"]["color"], "#FF0000");
        assert_eq!(cmd["data"]["position"], 0);
    }

    #[test]
    fn test_create_comment_command() {
        let issue_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let cmd = create_comment_command(
            issue_id,
            "Test comment",
            Some("markdown"),
            workspace_id,
            user_id,
        );

        assert_eq!(cmd["type"], "create_comment");
        assert_eq!(cmd["issue_id"], issue_id.to_string());
        assert_eq!(cmd["data"]["content"], "Test comment");
        assert_eq!(cmd["data"]["content_type"], "markdown");
    }

    #[test]
    fn test_update_workspace_member_command() {
        let target_user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let requesting_user_id = Uuid::new_v4();

        let cmd = update_workspace_member_command(
            target_user_id,
            "admin",
            workspace_id,
            requesting_user_id,
        );

        assert_eq!(cmd["type"], "update_workspace_member");
        assert_eq!(cmd["user_id"], target_user_id.to_string());
        assert_eq!(cmd["data"]["role"], "admin");
    }

    #[test]
    fn test_test_fixture_new() {
        let fixture = TestFixture::new();

        assert!(fixture.workspace_id != Uuid::nil());
        assert!(fixture.team_id != Uuid::nil());
        assert!(fixture.user_id != Uuid::nil());
        assert!(fixture.admin_user_id != Uuid::nil());
        assert!(fixture.member_ws.is_none());
        assert!(fixture.admin_ws.is_none());
    }
}
