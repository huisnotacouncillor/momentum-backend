use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{self, Output, ToSql};
use serde::{Deserialize, Serialize};
use std::io::Write;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum WorkflowStateCategory {
    Backlog,
    Unstarted,
    Started,
    Completed,
    Canceled,
    Triage,
}

impl WorkflowStateCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowStateCategory::Backlog => "backlog",
            WorkflowStateCategory::Unstarted => "unstarted",
            WorkflowStateCategory::Started => "started",
            WorkflowStateCategory::Completed => "completed",
            WorkflowStateCategory::Canceled => "canceled",
            WorkflowStateCategory::Triage => "triage",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "backlog" => WorkflowStateCategory::Backlog,
            "unstarted" => WorkflowStateCategory::Unstarted,
            "started" => WorkflowStateCategory::Started,
            "completed" => WorkflowStateCategory::Completed,
            "canceled" => WorkflowStateCategory::Canceled,
            "triage" => WorkflowStateCategory::Triage,
            _ => WorkflowStateCategory::Backlog,
        }
    }
}

impl std::fmt::Display for WorkflowStateCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for WorkflowStateCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(WorkflowStateCategory::from_str(&s))
    }
}

impl serde::Serialize for WorkflowStateCategory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl ToSql<diesel::sql_types::Text, Pg> for WorkflowStateCategory {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        let lowercase_str = match self {
            WorkflowStateCategory::Backlog => "backlog",
            WorkflowStateCategory::Unstarted => "unstarted",
            WorkflowStateCategory::Started => "started",
            WorkflowStateCategory::Completed => "completed",
            WorkflowStateCategory::Canceled => "canceled",
            WorkflowStateCategory::Triage => "triage",
        };
        out.write_all(lowercase_str.as_bytes())
            .map(|_| serialize::IsNull::No)
            .map_err(Into::into)
    }
}

impl FromSql<diesel::sql_types::Text, Pg> for WorkflowStateCategory {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        match std::str::from_utf8(bytes.as_bytes())? {
            "backlog" => Ok(WorkflowStateCategory::Backlog),
            "unstarted" => Ok(WorkflowStateCategory::Unstarted),
            "started" => Ok(WorkflowStateCategory::Started),
            "completed" => Ok(WorkflowStateCategory::Completed),
            "canceled" => Ok(WorkflowStateCategory::Canceled),
            "triage" => Ok(WorkflowStateCategory::Triage),
            _ => Ok(WorkflowStateCategory::Backlog),
        }
    }
}

// Workflow models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = crate::schema::workflows)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Workflow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub team_id: Uuid,
    pub is_default: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::workflows)]
pub struct NewWorkflow {
    pub name: String,
    pub description: Option<String>,
    pub team_id: Uuid,
    pub is_default: bool,
}

#[derive(AsChangeset, Default)]
#[diesel(table_name = crate::schema::workflows)]
pub struct UpdateWorkflow {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub is_default: Option<bool>,
}

// WorkflowState models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = crate::schema::workflow_states)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WorkflowState {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    #[serde(deserialize_with = "deserialize_category")]
    #[serde(serialize_with = "serialize_category")]
    pub category: WorkflowStateCategory,
    pub position: i32,
    pub is_default: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn deserialize_category<'de, D>(deserializer: D) -> Result<WorkflowStateCategory, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(WorkflowStateCategory::from_str(&s))
}

fn serialize_category<S>(category: &WorkflowStateCategory, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(category.as_str())
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::workflow_states)]
pub struct NewWorkflowState {
    pub workflow_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    #[diesel(column_name = category)]
    pub category: WorkflowStateCategory,
    pub position: i32,
    pub is_default: bool,
}

#[derive(AsChangeset, Default)]
#[diesel(table_name = crate::schema::workflow_states)]
pub struct UpdateWorkflowState {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub color: Option<Option<String>>,
    pub category: Option<WorkflowStateCategory>,
    pub position: Option<i32>,
    pub is_default: Option<bool>,
}

// WorkflowTransition models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = crate::schema::workflow_transitions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WorkflowTransition {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub from_state_id: Option<Uuid>,
    pub to_state_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::workflow_transitions)]
pub struct NewWorkflowTransition {
    pub workflow_id: Uuid,
    pub from_state_id: Option<Uuid>,
    pub to_state_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
}

// Request/Response models
#[derive(Deserialize, Serialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Deserialize, Serialize)]
pub struct CreateWorkflowStateRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: WorkflowStateCategory,
    pub position: i32,
    pub is_default: Option<bool>,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateWorkflowStateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: Option<WorkflowStateCategory>,
    pub position: Option<i32>,
    pub is_default: Option<bool>,
}

#[derive(Serialize, Clone)]
pub struct WorkflowResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub team_id: Uuid,
    pub is_default: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub states: Vec<WorkflowStateResponse>,
}

#[derive(Serialize, Clone)]
pub struct WorkflowStateResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    #[serde(serialize_with = "serialize_category")]
    pub category: WorkflowStateCategory,
    pub position: i32,
    pub is_default: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Clone)]
pub struct WorkflowTransitionResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub from_state_id: Option<Uuid>,
    pub to_state_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Clone)]
pub struct IssueTransitionResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub from_state_id: Option<Uuid>,
    pub to_state_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub from_state: Option<WorkflowStateResponse>,
    pub to_state: WorkflowStateResponse,
}

impl From<Workflow> for WorkflowResponse {
    fn from(workflow: Workflow) -> Self {
        WorkflowResponse {
            id: workflow.id,
            name: workflow.name,
            description: workflow.description,
            team_id: workflow.team_id,
            is_default: workflow.is_default,
            created_at: workflow.created_at,
            updated_at: workflow.updated_at,
            states: Vec::new(),
        }
    }
}

impl From<WorkflowState> for WorkflowStateResponse {
    fn from(state: WorkflowState) -> Self {
        WorkflowStateResponse {
            id: state.id,
            workflow_id: state.workflow_id,
            name: state.name,
            description: state.description,
            color: state.color,
            category: state.category,
            position: state.position,
            is_default: state.is_default,
            created_at: state.created_at,
            updated_at: state.updated_at,
        }
    }
}

impl From<WorkflowTransition> for WorkflowTransitionResponse {
    fn from(transition: WorkflowTransition) -> Self {
        WorkflowTransitionResponse {
            id: transition.id,
            workflow_id: transition.workflow_id,
            from_state_id: transition.from_state_id,
            to_state_id: transition.to_state_id,
            name: transition.name,
            description: transition.description,
            created_at: transition.created_at,
        }
    }
}

// Default workflow states data
pub struct DefaultWorkflowState {
    pub name: String,
    pub description: String,
    pub color: String,
    pub category: WorkflowStateCategory,
    pub position: i32,
    pub is_default: bool,
}

impl DefaultWorkflowState {
    pub fn get_default_states() -> Vec<Self> {
        vec![
            DefaultWorkflowState {
                name: "Backlog".to_string(),
                description: "Issues that are not yet prioritized".to_string(),
                color: "#999999".to_string(),
                category: WorkflowStateCategory::Backlog,
                position: 1,
                is_default: true,
            },
            DefaultWorkflowState {
                name: "Duplicated".to_string(),
                description: "Duplicated Issue".to_string(),
                color: "#333333".to_string(),
                category: WorkflowStateCategory::Canceled,
                position: 0,
                is_default: false,
            },
            DefaultWorkflowState {
                name: "Canceled".to_string(),
                description: "Canceled or invalid issues".to_string(),
                color: "#333333".to_string(),
                category: WorkflowStateCategory::Canceled,
                position: 1,
                is_default: false,
            },
            DefaultWorkflowState {
                name: "Done".to_string(),
                description: "Completed issues".to_string(),
                color: "#0082FF".to_string(),
                category: WorkflowStateCategory::Completed,
                position: 1,
                is_default: false,
            },
            DefaultWorkflowState {
                name: "In Progress".to_string(),
                description: "Issues currently being worked on".to_string(),
                color: "#F1BF00".to_string(),
                category: WorkflowStateCategory::Started,
                position: 1,
                is_default: false,
            },
            DefaultWorkflowState {
                name: "In Review".to_string(),
                description: "Issues ready for review".to_string(),
                color: "#82E0AA".to_string(),
                category: WorkflowStateCategory::Started,
                position: 2,
                is_default: false,
            },
            DefaultWorkflowState {
                name: "Todo".to_string(),
                description: "Issues that are ready to be worked on".to_string(),
                color: "#999999".to_string(),
                category: WorkflowStateCategory::Unstarted,
                position: 1,
                is_default: false,
            },
        ]
    }
}
