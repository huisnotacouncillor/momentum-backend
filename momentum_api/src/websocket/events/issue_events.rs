//! WebSocket Issue Events
//!
//! This module defines the event types for issue-related WebSocket notifications.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::WebSocketEvent;

/// Issue event types for WebSocket notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IssueEvent {
    /// Issue was created
    Created {
        issue: crate::db::models::issue::IssueResponse,
        workspace_id: Uuid,
    },
    /// Issue was updated
    Updated {
        issue: crate::db::models::issue::IssueResponse,
        changes: Vec<String>,
        workspace_id: Uuid,
    },
    /// Issue was deleted
    Deleted { issue_id: Uuid },
    /// Issue status changed
    StatusChanged {
        issue_id: Uuid,
        old_state: String,
        new_state: String,
    },
    /// Issue assignment changed
    Assigned {
        issue_id: Uuid,
        assignee_id: Option<Uuid>,
    },
}

impl WebSocketEvent for IssueEvent {
    fn event_type(&self) -> super::EventType {
        match self {
            IssueEvent::Created { .. } => super::EventType::Business,
            IssueEvent::Updated { .. } => super::EventType::Business,
            IssueEvent::Deleted { .. } => super::EventType::Business,
            IssueEvent::StatusChanged { .. } => super::EventType::Business,
            IssueEvent::Assigned { .. } => super::EventType::Business,
        }
    }

    fn user_id(&self) -> Option<Uuid> {
        match self {
            IssueEvent::Created { issue, .. } => Some(issue.creator_id),
            IssueEvent::Updated { issue, .. } => Some(issue.creator_id),
            IssueEvent::Deleted { .. } => None,
            IssueEvent::StatusChanged { .. } => None,
            IssueEvent::Assigned { .. } => None,
        }
    }

    fn connection_id(&self) -> Option<String> {
        None
    }

    fn workspace_id(&self) -> Option<Uuid> {
        match self {
            IssueEvent::Created { workspace_id, .. } => Some(*workspace_id),
            IssueEvent::Updated { workspace_id, .. } => Some(*workspace_id),
            IssueEvent::Deleted { .. } => None,
            IssueEvent::StatusChanged { .. } => None,
            IssueEvent::Assigned { .. } => None,
        }
    }

    fn should_broadcast(&self) -> bool {
        true
    }
}

impl IssueEvent {
    /// Returns the event type name for this issue event
    pub fn event_name(&self) -> &'static str {
        match self {
            IssueEvent::Created { .. } => "issue.created",
            IssueEvent::Updated { .. } => "issue.updated",
            IssueEvent::Deleted { .. } => "issue.deleted",
            IssueEvent::StatusChanged { .. } => "issue.status_changed",
            IssueEvent::Assigned { .. } => "issue.assigned",
        }
    }
}
