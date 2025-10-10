use uuid::Uuid;

use crate::{
    error::AppError, services::context::RequestContext, services::issues_service::IssuesService,
};

use super::types::*;

pub struct IssueHandlers;

impl IssueHandlers {
    pub async fn handle_create_issue(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        data: CreateIssueCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let issue = IssuesService::create_from_ws_command(&mut conn, &ctx, &data)?;
        Ok(serde_json::to_value(issue).unwrap())
    }

    pub async fn handle_update_issue(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        issue_id: Uuid,
        data: UpdateIssueCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let issue = IssuesService::update_from_ws_command(&mut conn, &ctx, issue_id, &data)?;
        Ok(serde_json::to_value(issue).unwrap())
    }

    pub async fn handle_delete_issue(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        issue_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        IssuesService::delete(&mut conn, &ctx, issue_id)?;
        Ok(serde_json::json!({"deleted": true, "issue_id": issue_id}))
    }

    pub async fn handle_query_issues(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        filters: IssueFilters,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let issues = IssuesService::list_from_ws_command(&mut conn, &ctx, &filters)?;
        Ok(serde_json::to_value(issues).unwrap())
    }

    pub async fn handle_get_issue(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        issue_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let issue = IssuesService::get_by_id(&mut conn, &ctx, issue_id)?;
        Ok(serde_json::to_value(issue).unwrap())
    }
}
