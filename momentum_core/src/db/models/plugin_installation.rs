use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::plugin_installations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PluginInstallation {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub plugin_id: String,
    pub config: serde_json::Value,
    pub status: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub enabled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::plugin_installations)]
pub struct NewPluginInstallation {
    pub workspace_id: Uuid,
    pub plugin_id: String,
    pub config: serde_json::Value,
    pub status: String,
}
