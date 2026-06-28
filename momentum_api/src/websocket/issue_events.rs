//! WebSocket Issue Events
//!
//! This module defines the event types for issue-related WebSocket notifications.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Issue event types for WebSocket notifications
#[derive(Clone, Serialize)]
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
