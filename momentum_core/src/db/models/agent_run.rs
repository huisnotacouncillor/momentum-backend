use crate::schema::agent_runs;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::agent_runs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AgentRun {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub issue_id: Option<Uuid>,
    pub plugin_id: String,
    pub agent_id: String,
    pub status: String,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub tokens_input: Option<i32>,
    pub tokens_output: Option<i32>,
    pub duration_ms: Option<i32>,
    pub actor_id: Option<Uuid>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::agent_runs)]
pub struct NewAgentRun {
    pub workspace_id: Uuid,
    pub issue_id: Option<Uuid>,
    pub plugin_id: String,
    pub agent_id: String,
    pub status: String,
    pub input: Option<serde_json::Value>,
    pub actor_id: Option<Uuid>,
}
