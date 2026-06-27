use crate::db::enums::IssuePriority;
use serde::Deserialize;
use uuid::Uuid;

/// Request to create a new issue
#[derive(Debug, Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Uuid,
    pub priority: Option<IssuePriority>,
    pub assignee_id: Option<Uuid>,
    pub reporter_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
    pub cycle_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
}

/// Request to update an existing issue
#[derive(Debug, Deserialize)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub priority: Option<IssuePriority>,
    pub assignee_id: Option<Uuid>,
    pub reporter_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
}