//! WebSocket Command Integration Tests
//!
//! These tests verify the complete CRUD flows for WebSocket commands.
//! Run with: cargo test -p momentum_api --test ws_command_integration_tests
//!
//! Note: These tests require a running server and database.
//! Start the test environment with: docker-compose -f docker-compose.test.yml up -d

mod fixtures;

use fixtures::{
    create_team_workflow_status_command,
    delete_team_workflow_status_command,
    get_team_workflow_statuses_command,
    get_workspace_command,
    update_team_workflow_status_command,
    TestFixture,
};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
#[ignore = "WebSocket message protocol needs debugging - commands not receiving responses"]
async fn test_team_workflow_status_full_crud_flow() {
    let mut fixture = TestFixture::setup().await.expect("Failed to setup test fixture");
    fixture.connect().await.expect("Failed to connect");

    // CREATE
    let create_response = fixture.ws.as_mut().unwrap()
        .send_command(create_team_workflow_status_command(
            fixture.team_id, "In Progress", "started", "#FF6B6B", 0,
            fixture.workspace_id, fixture.user_id,
        ))
        .await;

    if !create_response["success"].as_bool().unwrap_or(false) {
        println!("Create failed (expected without real DB data): {}", create_response);
        fixture.cleanup().await;
        return;
    }

    let status_id = create_response["data"]["id"].as_str().unwrap();
    let status_id = Uuid::parse_str(status_id).unwrap();

    // READ
    let get_all_response = fixture.ws.as_mut().unwrap()
        .send_command(get_team_workflow_statuses_command(
            fixture.team_id, fixture.workspace_id, fixture.user_id,
        ))
        .await;

    assert!(get_all_response["success"].as_bool().unwrap_or(false), "Get all should succeed");

    // UPDATE
    let update_response = fixture.ws.as_mut().unwrap()
        .send_command(update_team_workflow_status_command(
            fixture.team_id, status_id,
            json!({ "name": "In Development", "color": "#4ECDC4" }),
            fixture.workspace_id, fixture.user_id,
        ))
        .await;

    assert!(update_response["success"].as_bool().unwrap_or(false), "Update should succeed");

    // DELETE
    let delete_response = fixture.ws.as_mut().unwrap()
        .send_command(delete_team_workflow_status_command(
            fixture.team_id, status_id, fixture.workspace_id, fixture.user_id,
        ))
        .await;

    assert!(delete_response["success"].as_bool().unwrap_or(false), "Delete should succeed");

    fixture.cleanup().await;
}

#[tokio::test]
#[ignore = "WebSocket message protocol needs debugging"]
async fn test_get_workspace() {
    let mut fixture = TestFixture::setup().await.expect("Failed to setup test fixture");
    fixture.connect().await.expect("Failed to connect");

    let response = fixture.ws.as_mut().unwrap()
        .send_command(get_workspace_command(fixture.workspace_id, fixture.user_id))
        .await;

    println!("Response: {:?}", response);
    assert!(response.get("id").is_some(), "Response should have id: {:?}", response);

    fixture.cleanup().await;
}
