//! Issue command handlers

use std::sync::Arc;
use uuid::Uuid;
use momentum_core::db::DbPool;
use momentum_core::db::enums::IssuePriority;
use momentum_core::error::AppError;
use momentum_core::services::issues_service::{IssuesService, IssueFilters as ServiceIssueFilters};
use super::types::*;

pub struct IssueHandlers;

impl IssueHandlers {
    pub async fn handle_create_issue(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
        data: CreateIssueCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        // Convert priority from String to IssuePriority enum
        let priority = data.priority.as_ref().map(|p| match p.to_lowercase().as_str() {
            "none" => IssuePriority::None,
            "low" => IssuePriority::Low,
            "medium" => IssuePriority::Medium,
            "high" => IssuePriority::High,
            "urgent" => IssuePriority::Urgent,
            _ => IssuePriority::None,
        });

        let create_data = momentum_core::services::issues::types::CreateIssueRequest {
            title: data.title.clone(),
            description: data.description.clone(),
            team_id: data.team_id,
            project_id: data.project_id,
            priority,
            assignee_id: data.assignee_id,
            reporter_id: None,
            workflow_id: data.workflow_id,
            workflow_state_id: data.workflow_state_id,
            cycle_id: data.cycle_id,
            label_ids: data.label_ids,
            parent_issue_id: data.parent_issue_id,
        };

        let service = IssuesService::new();
        let result = service.create(&mut conn, &ctx, &create_data).await?;
        Ok(serde_json::json!(result))
    }

    pub async fn handle_update_issue(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
        issue_id: Uuid,
        data: UpdateIssueCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        // Convert priority from String to IssuePriority enum
        let priority = data.priority.and_then(|p| {
            match p.to_lowercase().as_str() {
                "none" => Some(IssuePriority::None),
                "low" => Some(IssuePriority::Low),
                "medium" => Some(IssuePriority::Medium),
                "high" => Some(IssuePriority::High),
                "urgent" => Some(IssuePriority::Urgent),
                _ => None,
            }
        });

        let update_data = momentum_core::services::issues::types::UpdateIssueRequest {
            title: data.title,
            description: data.description,
            project_id: data.project_id,
            team_id: data.team_id,
            priority,
            assignee_id: data.assignee_id,
            reporter_id: None,
            workflow_id: data.workflow_id,
            workflow_state_id: data.workflow_state_id,
            cycle_id: data.cycle_id,
            label_ids: data.label_ids,
        };

        let service = IssuesService::new();
        let result = service.update(&mut conn, &ctx, issue_id, &update_data).await?;
        Ok(serde_json::json!(result))
    }

    pub async fn handle_delete_issue(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
        issue_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;
        let service = IssuesService::new();
        service.delete(&mut conn, &ctx, issue_id)?;
        Ok(serde_json::json!({ "deleted": true, "issue_id": issue_id }))
    }

    pub async fn handle_query_issues(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
        filters: IssueFilters,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        // Convert priority from String to IssuePriority enum
        let priority = filters.priority.and_then(|p| {
            match p.to_lowercase().as_str() {
                "none" => Some(IssuePriority::None),
                "low" => Some(IssuePriority::Low),
                "medium" => Some(IssuePriority::Medium),
                "high" => Some(IssuePriority::High),
                "urgent" => Some(IssuePriority::Urgent),
                _ => None,
            }
        });

        // Convert to service layer filters
        let service_filters = ServiceIssueFilters {
            team_id: filters.team_id,
            project_id: filters.project_id,
            assignee_id: filters.assignee_id,
            priority,
            search: filters.search,
            limit: filters.limit.map(|l| l as i64),
            cursor: filters.cursor,
        };

        let service = IssuesService::new();
        let result = service.list(&mut conn, &ctx, &service_filters)?;

        Ok(serde_json::json!({
            "items": result.items,
            "next_cursor": result.next_cursor,
            "has_more": result.has_more,
        }))
    }

    pub async fn handle_get_issue(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
        issue_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;
        let service = IssuesService::new();
        let issue = service.get_by_id(&mut conn, &ctx, issue_id)?;
        Ok(serde_json::json!(issue))
    }

    pub async fn handle_query_issue_priorities(
        _ctx: momentum_core::services::context::RequestContext,
    ) -> Result<serde_json::Value, AppError> {
        // Return the list of valid issue priorities
        let priorities = vec![
            "none",
            "low",
            "medium",
            "high",
            "urgent",
        ];
        Ok(serde_json::json!(priorities))
    }
}
