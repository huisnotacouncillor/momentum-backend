use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::automation_rules)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AutomationRule {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub team_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub trigger_type: String,
    pub trigger_config: Option<serde_json::Value>,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::automation_rules)]
pub struct NewAutomationRule {
    pub workspace_id: Uuid,
    pub team_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub trigger_type: String,
    pub trigger_config: Option<serde_json::Value>,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = crate::schema::automation_rules)]
pub struct UpdateAutomationRule {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub is_enabled: Option<bool>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<Option<serde_json::Value>>,
    pub conditions: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
}

// ============ Trigger Types ============

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    IssueCreated,
    IssueUpdated,
    IssueStatusChanged,
    IssueAssigned,
}

impl TriggerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerType::IssueCreated => "issue_created",
            TriggerType::IssueUpdated => "issue_updated",
            TriggerType::IssueStatusChanged => "issue_status_changed",
            TriggerType::IssueAssigned => "issue_assigned",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "issue_created" => Some(TriggerType::IssueCreated),
            "issue_updated" => Some(TriggerType::IssueUpdated),
            "issue_status_changed" => Some(TriggerType::IssueStatusChanged),
            "issue_assigned" => Some(TriggerType::IssueAssigned),
            _ => None,
        }
    }
}

// ============ Condition ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: String,
    pub value: serde_json::Value,
}

// ============ Action ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "config")]
pub enum Action {
    TransitionState { state_id: Uuid },
    AddLabel { label_id: Uuid },
    RemoveLabel { label_id: Uuid },
    AssignTo { user_id: Uuid },
    SetPriority { priority: String },
}
