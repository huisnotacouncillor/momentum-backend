use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::issue_field_values)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct IssueFieldValue {
    pub issue_id: Uuid,
    pub field_id: Uuid,
    pub value: serde_json::Value,
    pub text_value: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::issue_field_values)]
pub struct NewIssueFieldValue {
    pub issue_id: Uuid,
    pub field_id: Uuid,
    pub value: serde_json::Value,
    pub text_value: Option<String>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = crate::schema::issue_field_values)]
pub struct IssueFieldValueChangeset {
    pub value: serde_json::Value,
    pub text_value: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
