use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    db::models::issue::{Issue, NewIssue},
    db::repositories::issues::IssueRepo,
    db::enums::IssuePriority,
    error::AppError,
    services::context::RequestContext,
    validation::issue::{validate_create_issue, validate_update_issue},
};

pub struct IssuesService;

impl IssuesService {
    pub fn list(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        filters: &IssueFilters,
    ) -> Result<Vec<Issue>, AppError> {
        let mut query = IssueRepo::list_by_workspace(conn, ctx.workspace_id)?;

        // Apply filters
        if let Some(team_id) = filters.team_id {
            query.retain(|issue| issue.team_id == team_id);
        }

        if let Some(project_id) = filters.project_id {
            query.retain(|issue| issue.project_id == Some(project_id));
        }

        if let Some(assignee_id) = filters.assignee_id {
            query.retain(|issue| issue.assignee_id == Some(assignee_id));
        }

        if let Some(priority) = &filters.priority {
            query.retain(|issue| issue.priority == format!("{:?}", priority));
        }

        if let Some(search) = &filters.search {
            query.retain(|issue| issue.title.to_lowercase().contains(&search.to_lowercase()));
        }

        Ok(query)
    }

    pub fn create(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        req: &crate::routes::issues::CreateIssueRequest,
    ) -> Result<Issue, AppError> {
        validate_create_issue(&req.title, &req.description, &req.team_id)?;

        let _now = Utc::now().naive_utc();
        let new_issue = NewIssue {
            project_id: req.project_id,
            cycle_id: req.cycle_id,
            creator_id: ctx.user_id,
            assignee_id: req.assignee_id,
            parent_issue_id: req.parent_issue_id,
            title: req.title.clone(),
            description: req.description.clone(),
            priority: req.priority.as_ref().map(|p| format!("{:?}", p)),
            is_changelog_candidate: Some(false),
            team_id: req.team_id,
            workflow_id: req.workflow_id,
            workflow_state_id: req.workflow_state_id,
        };

        IssueRepo::insert(conn, &new_issue)
            .map_err(|e| AppError::internal(&format!("Failed to create issue: {}", e)))
    }

    pub fn update(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        issue_id: Uuid,
        changes: &crate::routes::issues::UpdateIssueRequest,
    ) -> Result<Issue, AppError> {
        validate_update_issue(&changes.title, &changes.description)?;

        // Ensure issue exists in workspace
        let existing = IssueRepo::find_by_id_in_workspace(conn, ctx.workspace_id, issue_id)?;
        if existing.is_none() {
            return Err(AppError::not_found("issue"));
        }

        let updated = IssueRepo::update_fields(
            conn,
            issue_id,
            (
                changes.title.clone(),
                changes.description.clone(),
                changes.project_id,
                changes.team_id,
                changes.priority.as_ref().map(|p| format!("{:?}", p)),
                changes.assignee_id,
                changes.workflow_id,
                changes.workflow_state_id,
                changes.cycle_id,
            ),
        )?;
        Ok(updated)
    }

    pub fn delete(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        issue_id: Uuid,
    ) -> Result<(), AppError> {
        // Ensure issue exists in workspace
        let existing = IssueRepo::find_by_id_in_workspace(conn, ctx.workspace_id, issue_id)?;
        if existing.is_none() {
            return Err(AppError::not_found("issue"));
        }

        IssueRepo::delete_by_id(conn, issue_id)
            .map_err(|e| AppError::internal(&format!("Failed to delete issue: {}", e)))?;

        Ok(())
    }

    pub fn get_by_id(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        issue_id: Uuid,
    ) -> Result<Issue, AppError> {
        IssueRepo::find_by_id_in_workspace(conn, ctx.workspace_id, issue_id)?
            .ok_or_else(|| AppError::not_found("issue"))
    }
}

#[derive(Debug)]
pub struct IssueFilters {
    pub team_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub priority: Option<IssuePriority>,
    pub search: Option<String>,
}
