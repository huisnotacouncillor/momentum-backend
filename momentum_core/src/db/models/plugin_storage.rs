use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::plugin_storage)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PluginStorageEntry {
    pub plugin_id: String,
    pub workspace_id: Uuid,
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::plugin_storage)]
pub struct NewPluginStorage {
    pub plugin_id: String,
    pub workspace_id: Uuid,
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = crate::schema::plugin_storage)]
pub struct PluginStorageChangeset {
    pub value: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
