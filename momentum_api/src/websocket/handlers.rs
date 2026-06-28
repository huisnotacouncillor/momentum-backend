//! WebSocket Issue Events Handler
//!
//! This module provides handlers for issue-related WebSocket events.

use std::sync::Arc;

use axum::extract::ws::WebSocket;

use super::events::issue_events::IssueEvent;
use super::manager::{WebSocketManager, WebSocketMessage};

/// Handle WebSocket connections for issue events subscription
pub async fn handle_issue_events(
    socket: WebSocket,
    manager: Arc<WebSocketManager>,
    state: Arc<crate::state::AppState>,
) {
    // Stub implementation for issue events WebSocket handling
    // This would handle:
    // - Subscribing to issue updates
    // - Unsubscribing from issue updates
    // - Broadcasting issue changes to subscribed clients

    info!("Issue events WebSocket handler called");
}

/// Subscribe a user to issue events for a workspace
pub async fn subscribe_to_issue_events(
    manager: &WebSocketManager,
    user_id: uuid::Uuid,
    workspace_id: uuid::Uuid,
) {
    let topic = format!("issues:{}", workspace_id);
    manager.subscribe(user_id, topic).await;
}

/// Unsubscribe a user from issue events for a workspace
pub async fn unsubscribe_from_issue_events(
    manager: &WebSocketManager,
    user_id: uuid::Uuid,
    workspace_id: uuid::Uuid,
) {
    let topic = format!("issues:{}", workspace_id);
    manager.unsubscribe(user_id, topic).await;
}

/// Broadcast an issue created event
pub async fn broadcast_issue_created(
    manager: &WebSocketManager,
    workspace_id: uuid::Uuid,
    issue: crate::db::models::issue::IssueResponse,
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
    issue: crate::db::models::issue::IssueResponse,
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
