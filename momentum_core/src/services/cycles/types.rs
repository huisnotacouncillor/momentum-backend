use crate::db::enums::CycleStatus;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to create a new cycle
#[derive(Debug, Deserialize)]
pub struct CreateCycleRequest {
    pub team_id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub description: Option<String>,
    pub goal: Option<String>,
}

/// Request to update an existing cycle
#[derive(Debug, Deserialize)]
pub struct UpdateCycleRequest {
    pub team_id: Option<Uuid>,
    pub name: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub goal: Option<String>,
}

/// Cycle statistics response
#[derive(Debug, Serialize)]
pub struct CycleStats {
    pub id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: CycleStatus,
    pub total_issues: i64,
    pub completed_issues: i64,
    pub in_progress_issues: i64,
    pub todo_issues: i64,
    pub completion_rate: f64,
    pub days_remaining: i32,
    pub is_overdue: bool,
}