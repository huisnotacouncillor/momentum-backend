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
    pub project_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub creator_id: Uuid,
    pub assignee_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
    pub issue_number: i32,
    pub title: String,
    pub description: Option<String>,
    #[serde(skip)]
    pub status: String,
    #[serde(skip)]
    pub priority: String,
    pub is_changelog_candidate: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub team_id: Uuid,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::issues)]
pub struct NewIssue {
    pub project_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub creator_id: Uuid,
    pub assignee_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub is_changelog_candidate: Option<bool>,
    pub team_id: Uuid,
}

// Issue update model
#[derive(AsChangeset, Default)]
#[diesel(table_name = crate::schema::issues)]
pub struct UpdateIssue {
    pub project_id: Option<Option<Uuid>>,
    pub cycle_id: Option<Option<Uuid>>,
    pub assignee_id: Option<Option<Uuid>>,
    pub parent_issue_id: Option<Option<Uuid>>,
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub is_changelog_candidate: Option<bool>,
    pub team_id: Option<Uuid>,
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

// DTOs for API requests and responses
#[derive(Deserialize, Serialize)]
pub struct CreateIssueRequest {
    pub team_id: Uuid,
    pub project_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub is_changelog_candidate: bool,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateIssueRequest {
    pub team_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<IssueStatus>,
    pub priority: Option<IssuePriority>,
    pub is_changelog_candidate: Option<bool>,
}

#[derive(Serialize, Clone)]
pub struct IssueResponse {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub creator_id: Uuid,
    pub assignee_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
    pub issue_number: i32,
    pub title: String,
    pub description: Option<String>,
    #[serde(serialize_with = "serialize_status")]
    pub status: IssueStatus,
    #[serde(serialize_with = "serialize_priority")]
    pub priority: IssuePriority,
    pub is_changelog_candidate: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub team_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<crate::db::models::auth::UserBasicInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_issue: Option<Box<IssueResponse>>, // boxed to avoid infinite size
    #[serde(default)]
    pub child_issues: Vec<IssueResponse>,
}

fn serialize_status<S>(status: &IssueStatus, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let status_str = match status {
        IssueStatus::Backlog => "backlog",
        IssueStatus::Todo => "todo",
        IssueStatus::InProgress => "in_progress",
        IssueStatus::InReview => "in_review",
        IssueStatus::Done => "done",
        IssueStatus::Canceled => "canceled",
    };
    serializer.serialize_str(status_str)
}

fn serialize_priority<S>(priority: &IssuePriority, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let priority_str = match priority {
        IssuePriority::None => "none",
        IssuePriority::Low => "low",
        IssuePriority::Medium => "medium",
        IssuePriority::High => "high",
        IssuePriority::Urgent => "urgent",
    };
    serializer.serialize_str(priority_str)
}

impl From<Issue> for IssueResponse {
    fn from(issue: Issue) -> Self {
        let status = match issue.status.as_str() {
            "backlog" => IssueStatus::Backlog,
            "todo" => IssueStatus::Todo,
            "in_progress" => IssueStatus::InProgress,
            "in_review" => IssueStatus::InReview,
            "done" => IssueStatus::Done,
            "canceled" => IssueStatus::Canceled,
            _ => IssueStatus::Todo, // default value
        };

        let priority = match issue.priority.as_str() {
            "none" => IssuePriority::None,
            "low" => IssuePriority::Low,
            "medium" => IssuePriority::Medium,
            "high" => IssuePriority::High,
            "urgent" => IssuePriority::Urgent,
            _ => IssuePriority::None, // default value
        };

        IssueResponse {
            id: issue.id,
            project_id: issue.project_id,
            cycle_id: issue.cycle_id,
            creator_id: issue.creator_id,
            assignee_id: issue.assignee_id,
            parent_issue_id: issue.parent_issue_id,
            issue_number: issue.issue_number,
            title: issue.title,
            description: issue.description,
            status,
            priority,
            is_changelog_candidate: issue.is_changelog_candidate,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            team_id: issue.team_id,
            assignee: None,
            parent_issue: None,
            child_issues: Vec::new(),
        }
    }
}
