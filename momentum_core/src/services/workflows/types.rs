use crate::db::models::workflow::WorkflowStateCategory;
use serde::Deserialize;
use uuid::Uuid;

/// Request to create a team default workflow state
#[derive(Debug, Deserialize)]
pub struct CreateTeamDefaultStateRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: WorkflowStateCategory,
    pub position: i32,
}

/// Request to update a team default workflow state
#[derive(Debug, Deserialize)]
pub struct UpdateTeamDefaultStateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: Option<WorkflowStateCategory>,
    pub position: Option<i32>,
}

/// Issue transition information
#[derive(Debug)]
pub struct IssueTransition {
    pub from_state_id: Option<Uuid>,
    pub to_state_id: Uuid,
    pub to_state_name: String,
    pub to_state_color: Option<String>,
}