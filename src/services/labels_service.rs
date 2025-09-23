use diesel::prelude::*;

use crate::{
    db::models::label::{Label, NewLabel},
    db::repositories::labels::LabelRepo,
    error::AppError,
    services::context::RequestContext,
    validation::label::validate_create_label,
};

pub struct LabelsService;

impl LabelsService {
    pub fn list(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        name_filter: Option<String>,
        level_filter: Option<crate::db::enums::LabelLevel>,
    ) -> Result<Vec<Label>, AppError> {
        // Use repository then filter in DB when possible
        use crate::schema::labels::dsl as l;
        let mut query = l::labels
            .filter(l::workspace_id.eq(ctx.workspace_id))
            .into_boxed();
        if let Some(name_like) = name_filter {
            let pattern = format!("%{}%", name_like);
            query = query.filter(l::name.like(pattern));
        }
        if let Some(level_val) = level_filter {
            query = query.filter(l::level.eq(level_val));
        }
        let results = query
            .order(l::created_at.desc())
            .load::<Label>(conn)?;
        Ok(results)
    }

    pub fn create(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        req: &crate::routes::labels::CreateLabelRequest,
    ) -> Result<Label, AppError> {
        validate_create_label(&req.name, &req.color)?;

        if LabelRepo::exists_by_name(conn, ctx.workspace_id, &req.name)? {
            return Err(AppError::conflict_with_code(
                "Label already exists",
                Some("name".to_string()),
                "LABEL_EXISTS",
            ));
        }

        let now = chrono::Utc::now().naive_utc();
        let new_label = NewLabel {
            workspace_id: ctx.workspace_id,
            name: req.name.clone(),
            color: req.color.clone(),
            level: req.level.clone(),
            created_at: now,
            updated_at: now,
        };

        let label = LabelRepo::insert(conn, &new_label)?;
        Ok(label)
    }

    pub fn update(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        label_id: uuid::Uuid,
        changes: &crate::routes::labels::UpdateLabelRequest,
    ) -> Result<Label, AppError> {
        // ensure label exists in workspace
        let existing = LabelRepo::find_by_id_in_workspace(conn, ctx.workspace_id, label_id)?;
        if existing.is_none() {
            return Err(AppError::not_found("label"));
        }
        // validate changes
        let update_changes = crate::validation::label::UpdateLabelChanges {
            name: changes.name.as_deref(),
            color: changes.color.as_deref(),
            level_present: changes.level.is_some(),
        };
        crate::validation::label::validate_update_label(&update_changes)?;

        // enforce name uniqueness if name changes
        if let Some(ref new_name) = changes.name {
            if LabelRepo::exists_by_name_excluding_id(conn, ctx.workspace_id, new_name, label_id)? {
                return Err(AppError::conflict_with_code(
                    "Label already exists",
                    Some("name".to_string()),
                    "LABEL_EXISTS",
                ));
            }
        }

        let updated = LabelRepo::update_fields(
            conn,
            label_id,
            (changes.name.clone(), changes.color.clone(), changes.level.clone()),
        )?;
        Ok(updated)
    }

    pub fn delete(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        label_id: uuid::Uuid,
    ) -> Result<(), AppError> {
        // ensure exists in workspace
        let existing = LabelRepo::find_by_id_in_workspace(conn, ctx.workspace_id, label_id)?;
        if existing.is_none() {
            return Err(AppError::not_found("label"));
        }
        LabelRepo::delete_by_id(conn, label_id)?;
        Ok(())
    }
}


