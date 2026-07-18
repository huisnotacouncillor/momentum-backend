//! WebSocket Issue Events Handler
//!
//! This module provides handlers for issue-related WebSocket events.

use std::sync::Arc;

use axum::extract::ws::WebSocket;
use tracing::info;

use super::issue_events::IssueEvent;
use super::manager::WebSocketManager;

/// Handle WebSocket connections for issue events subscription
pub async fn handle_issue_events(
    _socket: WebSocket,
    _manager: Arc<WebSocketManager>,
    _state: Arc<crate::state::AppState>,
) {
    // Stub implementation for issue events WebSocket handling
    // This would handle:
    // - Subscribing to issue updates
    // - Unsubscribing from issue updates
    // - Broadcasting issue changes to subscribed clients

    info!("Issue events WebSocket handler called");
}

/// Broadcast an issue created event
pub async fn broadcast_issue_created(
    manager: &WebSocketManager,
    workspace_id: uuid::Uuid,
    issue: serde_json::Value,
) {
    let event = IssueEvent::Created {
        issue,
        workspace_id,
    };
    manager.broadcast_issue_event(event).await;
}

/// Broadcast an issue updated event
pub async fn broadcast_issue_updated(
    manager: &WebSocketManager,
    workspace_id: uuid::Uuid,
    issue: serde_json::Value,
    changes: Vec<String>,
) {
    let event = IssueEvent::Updated {
        issue,
        changes,
        workspace_id,
    };
    manager.broadcast_issue_event(event).await;
}

/// Broadcast an issue deleted event
pub async fn broadcast_issue_deleted(manager: &WebSocketManager, issue_id: uuid::Uuid) {
    let event = IssueEvent::Deleted { issue_id };
    manager.broadcast_issue_event(event).await;
}

/// Broadcast an issue status changed event
pub async fn broadcast_issue_status_changed(
    manager: &WebSocketManager,
    workspace_id: uuid::Uuid,
    issue_id: uuid::Uuid,
    old_state: String,
    new_state: String,
) {
    let event = IssueEvent::StatusChanged {
        issue_id,
        old_state,
        new_state,
    };
    manager
        .broadcast_issue_event_to_workspace(workspace_id, event)
        .await;
}

/// Broadcast an issue assignment changed event
pub async fn broadcast_issue_assigned(
    manager: &WebSocketManager,
    workspace_id: uuid::Uuid,
    issue_id: uuid::Uuid,
    assignee_id: Option<uuid::Uuid>,
) {
    let event = IssueEvent::Assigned {
        issue_id,
        assignee_id,
    };
    manager
        .broadcast_issue_event_to_workspace(workspace_id, event)
        .await;
}
