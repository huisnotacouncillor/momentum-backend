use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Roadmap models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::roadmaps)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Roadmap {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::roadmaps)]
pub struct NewRoadmap {
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
}
