use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::plugin_audit)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PluginAuditRow {
    pub id: i64,
    pub plugin_id: String,
    pub workspace_id: Option<Uuid>,
    pub event: String,
    pub payload: Option<serde_json::Value>,
    pub actor_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::plugin_audit)]
pub struct NewPluginAudit {
    pub plugin_id: String,
    pub workspace_id: Option<Uuid>,
    pub event: String,
    pub payload: Option<serde_json::Value>,
    pub actor_id: Option<Uuid>,
}
