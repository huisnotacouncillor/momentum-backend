//! Label command handlers

use std::sync::Arc;
use uuid::Uuid;
use momentum_core::db::DbPool;
use momentum_core::error::AppError;
use momentum_core::services::context::RequestContext;
use momentum_core::services::labels_service::LabelsService;
use momentum_core::services::labels::types::{CreateLabelRequest, UpdateLabelRequest};
use super::types::*;

pub struct LabelHandlers;

impl LabelHandlers {
    pub async fn handle_create_label(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        data: CreateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let req = CreateLabelRequest {
            name: data.name,
            color: data.color,
            level: data.level,
        };

        let label = LabelsService::create(&mut conn, &ctx, &req)?;

        Ok(serde_json::json!(label))
    }

    pub async fn handle_update_label(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        label_id: Uuid,
        data: UpdateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let req = UpdateLabelRequest {
            name: data.name,
            color: data.color,
            level: data.level,
        };

        let label = LabelsService::update(&mut conn, &ctx, label_id, &req)?;

        Ok(serde_json::json!(label))
    }

    pub async fn handle_delete_label(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        label_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        LabelsService::delete(&mut conn, &ctx, label_id)?;

        Ok(serde_json::json!({
            "label_id": label_id.to_string(),
            "deleted": true
        }))
    }

    pub async fn handle_query_labels(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        filters: LabelFilters,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        let labels = LabelsService::list(
            &mut conn,
            &ctx,
            filters.name_pattern.clone(),
            filters.level,
        )?;

        Ok(serde_json::json!({
            "items": labels,
            "has_more": false,
            "next_cursor": null
        }))
    }

    pub async fn handle_get_label(
        db: &Arc<DbPool>,
        ctx: RequestContext,
        label_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;

        // Get single label by finding it in the list
        let labels = LabelsService::list(&mut conn, &ctx, None, None)?;
        let label = labels.into_iter()
            .find(|l| l.id == label_id)
            .ok_or_else(|| AppError::not_found("label"))?;

        Ok(serde_json::json!(label))
    }
}
