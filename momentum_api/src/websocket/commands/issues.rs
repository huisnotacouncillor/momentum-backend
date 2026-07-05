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
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _data: CreateIssueCommand,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Issue handlers not yet implemented"))
    }

    pub async fn handle_update_issue(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _issue_id: Uuid,
        _data: UpdateIssueCommand,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Issue handlers not yet implemented"))
    }

    pub async fn handle_delete_issue(
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _issue_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Issue handlers not yet implemented"))
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
        _db: &Arc<DbPool>,
        _ctx: momentum_core::services::context::RequestContext,
        _issue_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::internal("Issue handlers not yet implemented"))
    }
}
