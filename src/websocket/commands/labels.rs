use uuid::Uuid;

use crate::{
    error::AppError,
    services::context::RequestContext,
    validation::label::{UpdateLabelChanges, validate_create_label, validate_update_label},
};

use super::types::*;

pub struct LabelHandlers;

impl LabelHandlers {
    pub async fn handle_create_label(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        data: CreateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        validate_create_label(&data.name, &data.color)?;
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let req = crate::routes::labels::CreateLabelRequest {
            name: data.name,
            color: data.color,
            level: data.level,
        };
        let label = crate::services::labels_service::LabelsService::create(&mut conn, &ctx, &req)?;
        Ok(serde_json::to_value(&label).unwrap())
    }

    pub async fn handle_update_label(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        label_id: Uuid,
        data: UpdateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        let changes = UpdateLabelChanges {
            name: data.name.as_deref(),
            color: data.color.as_deref(),
            level_present: data.level.is_some(),
        };
        validate_update_label(&changes)?;
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let req = crate::routes::labels::UpdateLabelRequest {
            name: data.name,
            color: data.color,
            level: data.level,
        };
        let updated = crate::services::labels_service::LabelsService::update(
            &mut conn, &ctx, label_id, &req,
        )?;
        Ok(serde_json::to_value(&updated).unwrap())
    }

    pub async fn handle_delete_label(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        label_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        crate::services::labels_service::LabelsService::delete(&mut conn, &ctx, label_id)?;
        Ok(serde_json::json!({"deleted": true, "label_id": label_id}))
    }

    pub async fn handle_query_labels(
        db: &crate::db::DbPool,
        ctx: RequestContext,
        filters: LabelFilters,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let labels = crate::services::labels_service::LabelsService::list(
            &mut conn,
            &ctx,
            filters.name_pattern,
            filters.level,
        )?;
        Ok(serde_json::to_value(labels).unwrap())
    }
}
