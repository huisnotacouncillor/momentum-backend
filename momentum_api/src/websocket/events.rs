//! WebSocket event messages
//!
//! These are the events sent by the server to clients over WebSocket.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueCreatedEvent {
    pub issue_id: Uuid,
    pub project_id: Uuid,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueUpdatedEvent {
    pub issue_id: Uuid,
    pub changes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueDeletedEvent {
    pub issue_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketEvent {
    IssueCreated(IssueCreatedEvent),
    IssueUpdated(IssueUpdatedEvent),
    IssueDeleted(IssueDeletedEvent),
}
