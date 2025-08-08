use crate::db::enums::CycleStatus;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Cycle models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::cycles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Cycle {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub status: CycleStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::cycles)]
pub struct NewCycle {
    pub team_id: Uuid,
    pub name: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
}
