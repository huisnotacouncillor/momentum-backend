use crate::db::enums::{IssuePriority, IssueStatus};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Issue models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::issues)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Issue {
    pub id: Uuid,
    pub team_id: Uuid,
    pub project_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub creator_id: Uuid,
    pub assignee_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
    pub issue_number: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: IssueStatus,
    pub priority: IssuePriority,
    pub is_changelog_candidate: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::issues)]
pub struct NewIssue {
    pub team_id: Uuid,
    pub project_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub creator_id: Uuid,
    pub assignee_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub is_changelog_candidate: Option<bool>,
}

// Issue Label models (many-to-many relationship)
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::issue_labels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct IssueLabel {
    pub issue_id: Uuid,
    pub label_id: Uuid,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::issue_labels)]
pub struct NewIssueLabel {
    pub issue_id: Uuid,
    pub label_id: Uuid,
}