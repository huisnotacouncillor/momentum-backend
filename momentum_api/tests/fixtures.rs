//! Test Fixtures for WebSocket Integration Tests
//!
//! Run with: cargo test -p momentum_api --test ws_command_integration_tests
//! Note: These tests require a running server and database.

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use url::Url;
use uuid::Uuid;

const WEBSOCKET_URL: &str = "ws://127.0.0.1:8000/ws";
const TEST_JWT_SECRET: &str = "test-secret-key-for-integration-tests";

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

pub struct WebSocketTestConnection {
    pub sender: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        TungsteniteMessage,
    >,
    pub receiver: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    >,
}

impl WebSocketTestConnection {
    pub async fn send_command(&mut self, command: serde_json::Value) -> serde_json::Value {
        self.sender
            .send(TungsteniteMessage::Text(command.to_string()))
            .await
            .expect("Failed to send command");

        // Skip messages until we get a command response
        while let Ok(Some(msg)) = timeout(Duration::from_secs(5), self.receiver.next()).await {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    let resp: serde_json::Value = serde_json::from_str(&text).expect("Failed to parse response");
                    let msg_type = resp.get("message_type").and_then(|m| m.as_str()).unwrap_or("");
                    match msg_type {
                        "system_message" | "initial_data" | "user_joined" | "user_left" => {
                            eprintln!("[WS] Skipping message type: {}", msg_type);
                            continue;
                        }
                        _ => return resp,
                    }
                }
                Ok(TungsteniteMessage::Close(_)) => {
                    panic!("Connection closed by server")
                }
                _ => panic!("Unexpected message type"),
            }
        }
        panic!("No response received within timeout");
    }

    pub async fn close(mut self) {
        let _ = self.sender.close().await;
    }
}

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

// Test fixture with pre-generated IDs for testing
#[allow(dead_code)]
pub struct TestFixture {
    pub workspace_id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub admin_user_id: Uuid,
    pub ws: Option<WebSocketTestConnection>,
}

impl TestFixture {
    /// Setup with real user data
    pub async fn setup() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let unique = Uuid::new_v4().to_string()[..8].to_string();
        let email = format!("test_{}@example.com", unique);
        let username = format!("testuser_{}", unique);
        let password = "Testpass123!";

        // Register user
        let reg_resp = client
            .post("http://127.0.0.1:8000/auth/register")
            .json(&serde_json::json!({
                "email": email,
                "name": "Test User",
                "username": username,
                "password": password
            }))
            .send()
            .await?;

        if !reg_resp.status().is_success() {
            let body = reg_resp.text().await?;
            return Err(format!("Failed to register: {}", body).into());
        }

        let reg_data: serde_json::Value = reg_resp.json().await?;
        let user_id = reg_data["data"]["user"]["id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or("Failed to parse user_id")?;

        // Get token from register response (access_token field)
        let token = reg_data["data"]["access_token"]
            .as_str()
            .ok_or("No token in register response")?
            .to_string();

        // Create workspace
        let ws_resp = client
            .post("http://127.0.0.1:8000/workspaces")
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "name": format!("Test Workspace {}", unique),
                "url_key": format!("test_ws_{}", unique)
            }))
            .send()
            .await?;

        let ws_status = ws_resp.status();
        let ws_body = ws_resp.text().await?;
        if !ws_status.is_success() {
            return Err(format!("Workspace creation failed ({}): {}", ws_status, ws_body).into());
        }
        let ws_data: serde_json::Value = serde_json::from_str(&ws_body)
            .map_err(|e| format!("Failed to parse workspace response: {} - body: {}", e, ws_body))?;
        let workspace_id = ws_data["data"]["id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or("Failed to parse workspace_id")?;

        // Create team
        let team_resp = client
            .post("http://127.0.0.1:8000/teams")
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "name": format!("Test Team {}", unique),
                "team_key": format!("TT{}", unique.to_uppercase()),
                "is_private": false
            }))
            .send()
            .await?;

        let team_status = team_resp.status();
        let team_body = team_resp.text().await?;
        if !team_status.is_success() {
            return Err(format!("Team creation failed ({}): {}", team_status, team_body).into());
        }
        let team_data: serde_json::Value = serde_json::from_str(&team_body)
            .map_err(|e| format!("Failed to parse team response: {} - body: {}", e, team_body))?;
        let team_id = team_data["data"]["id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or("Failed to parse team_id")?;

        Ok(Self {
            workspace_id,
            team_id,
            user_id,
            admin_user_id: user_id,
            ws: None,
        })
    }

    pub fn new() -> Self {
        Self {
            workspace_id: Uuid::new_v4(),
            team_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            admin_user_id: Uuid::new_v4(),
            ws: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let conn = connect_websocket(
            self.user_id,
            "test_user",
            "test@example.com",
            self.workspace_id,
        )
        .await?;
        self.ws = Some(conn);
        Ok(())
    }

    pub async fn cleanup(&mut self) {
        if let Some(conn) = self.ws.take() {
            conn.close().await;
        }
    }
}

// ========================================================================
// Command Builders
// ========================================================================

fn base_command(command_type: &str, workspace_id: Uuid, user_id: Uuid, request_id: Option<String>) -> serde_json::Value {
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

pub fn create_team_workflow_status_command(
    team_id: Uuid, name: &str, category: &str, color: &str, position: i32,
    workspace_id: Uuid, user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("create_team_workflow_status", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd["data"] = json!({ "name": name, "category": category, "color": color, "position": position });
    cmd
}

pub fn get_team_workflow_statuses_command(team_id: Uuid, workspace_id: Uuid, user_id: Uuid) -> serde_json::Value {
    let mut cmd = base_command("get_team_workflow_statuses", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd
}

pub fn update_team_workflow_status_command(
    team_id: Uuid, status_id: Uuid, data: serde_json::Value,
    workspace_id: Uuid, user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("update_team_workflow_status", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd["status_id"] = serde_json::Value::String(status_id.to_string());
    cmd["data"] = data;
    cmd
}

pub fn delete_team_workflow_status_command(
    team_id: Uuid, status_id: Uuid, workspace_id: Uuid, user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("delete_team_workflow_status", workspace_id, user_id, None);
    cmd["team_id"] = serde_json::Value::String(team_id.to_string());
    cmd["status_id"] = serde_json::Value::String(status_id.to_string());
    cmd
}

#[allow(dead_code)]
pub fn create_comment_command(
    issue_id: Uuid, content: &str, content_type: Option<&str>,
    workspace_id: Uuid, user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("create_comment", workspace_id, user_id, None);
    cmd["issue_id"] = serde_json::Value::String(issue_id.to_string());
    cmd["data"] = json!({ "content": content, "content_type": content_type.unwrap_or("markdown") });
    cmd
}

#[allow(dead_code)]
pub fn query_comments_command(issue_id: Uuid, workspace_id: Uuid, user_id: Uuid) -> serde_json::Value {
    let mut cmd = base_command("query_comments", workspace_id, user_id, None);
    cmd["issue_id"] = serde_json::Value::String(issue_id.to_string());
    cmd
}

#[allow(dead_code)]
pub fn update_comment_command(
    issue_id: Uuid, comment_id: Uuid, content: &str,
    workspace_id: Uuid, user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("update_comment", workspace_id, user_id, None);
    cmd["issue_id"] = serde_json::Value::String(issue_id.to_string());
    cmd["comment_id"] = serde_json::Value::String(comment_id.to_string());
    cmd["data"] = json!({ "content": content });
    cmd
}

#[allow(dead_code)]
pub fn delete_comment_command(
    issue_id: Uuid, comment_id: Uuid, workspace_id: Uuid, user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("delete_comment", workspace_id, user_id, None);
    cmd["issue_id"] = serde_json::Value::String(issue_id.to_string());
    cmd["comment_id"] = serde_json::Value::String(comment_id.to_string());
    cmd
}

pub fn get_workspace_command(workspace_id: Uuid, user_id: Uuid) -> serde_json::Value {
    let mut cmd = base_command("get_workspace", workspace_id, user_id, None);
    cmd["workspace_id"] = serde_json::Value::String(workspace_id.to_string());
    cmd
}

#[allow(dead_code)]
pub fn get_workspace_member_command(
    user_id: Uuid, workspace_id: Uuid, requesting_user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("get_workspace_member", workspace_id, requesting_user_id, None);
    cmd["user_id"] = serde_json::Value::String(user_id.to_string());
    cmd
}

#[allow(dead_code)]
pub fn update_workspace_member_command(
    target_user_id: Uuid, role: &str, workspace_id: Uuid, requesting_user_id: Uuid,
) -> serde_json::Value {
    let mut cmd = base_command("update_workspace_member", workspace_id, requesting_user_id, None);
    cmd["user_id"] = serde_json::Value::String(target_user_id.to_string());
    cmd["data"] = json!({ "role": role });
    cmd
}

#[allow(dead_code)]
pub fn delete_workspace_member_command(
    target_user_id: Uuid, workspace_id: Uuid, requesting_user_id: Uuid,
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
        let token = create_test_jwt(Uuid::new_v4(), "test", "test@example.com", Uuid::new_v4());
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn test_base_command_structure() {
        let cmd = base_command("test", Uuid::new_v4(), Uuid::new_v4(), Some("req-123".to_string()));
        assert_eq!(cmd["type"], "test");
        assert_eq!(cmd["request_id"], "req-123");
    }

    #[test]
    fn test_test_fixture_new() {
        let f = TestFixture::new();
        assert!(f.workspace_id != Uuid::nil());
        assert!(f.team_id != Uuid::nil());
        assert!(f.ws.is_none());
    }
}
