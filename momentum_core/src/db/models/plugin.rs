use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::plugins)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: Option<String>,
    pub manifest: serde_json::Value,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::plugins)]
pub struct NewPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: Option<String>,
    pub manifest: serde_json::Value,
    pub status: String,
}
