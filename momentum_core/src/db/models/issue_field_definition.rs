use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::issue_field_definitions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct IssueFieldDefinition {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub plugin_id: String,
    pub field_key: String,
    pub label: String,
    pub field_type: String,
    pub options: Option<serde_json::Value>,
    pub required: bool,
    pub sort_order: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::issue_field_definitions)]
pub struct NewIssueFieldDefinition {
    pub workspace_id: Uuid,
    pub plugin_id: String,
    pub field_key: String,
    pub label: String,
    pub field_type: String,
    pub options: Option<serde_json::Value>,
    pub required: bool,
    pub sort_order: i32,
}
