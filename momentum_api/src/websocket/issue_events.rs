//! WebSocket Issue Events
//!
//! This module defines the event types for issue-related WebSocket notifications.

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use uuid::Uuid;

/// Issue event types for WebSocket notifications
#[derive(Clone)]
pub enum IssueEvent {
    /// Issue was created
    Created {
        issue: serde_json::Value,
        workspace_id: Uuid,
    },
    /// Issue was updated
    Updated {
        issue: serde_json::Value,
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

impl Serialize for IssueEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("IssueEvent", 2)?;
        match self {
            IssueEvent::Created { issue, workspace_id } => {
                state.serialize_field("type", "issue.created")?;
                state.serialize_field("data", issue)?;
            }
            IssueEvent::Updated { issue, changes, workspace_id } => {
                state.serialize_field("type", "issue.updated")?;
                #[derive(Serialize)]
                struct UpdatedData<'a> {
                    issue: &'a serde_json::Value,
                    changes: &'a Vec<String>,
                    workspace_id: &'a Uuid,
                }
                state.serialize_field(
                    "data",
                    &UpdatedData {
                        issue,
                        changes,
                        workspace_id,
                    },
                )?;
            }
            IssueEvent::Deleted { issue_id } => {
                state.serialize_field("type", "issue.deleted")?;
                state.serialize_field("data", issue_id)?;
            }
            IssueEvent::StatusChanged {
                issue_id,
                old_state,
                new_state,
            } => {
                state.serialize_field("type", "issue.status_changed")?;
                #[derive(Serialize)]
                struct StatusChangedData<'a> {
                    issue_id: &'a Uuid,
                    old_state: &'a str,
                    new_state: &'a str,
                }
                state.serialize_field(
                    "data",
                    &StatusChangedData {
                        issue_id,
                        old_state,
                        new_state,
                    },
                )?;
            }
            IssueEvent::Assigned {
                issue_id,
                assignee_id,
            } => {
                state.serialize_field("type", "issue.assigned")?;
                #[derive(Serialize)]
                struct AssignedData<'a> {
                    issue_id: &'a Uuid,
                    assignee_id: &'a Option<Uuid>,
                }
                state.serialize_field(
                    "data",
                    &AssignedData {
                        issue_id,
                        assignee_id,
                    },
                )?;
            }
        }
        state.end()
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
