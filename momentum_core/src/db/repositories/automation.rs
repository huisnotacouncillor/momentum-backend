use diesel::prelude::*;
use uuid::Uuid;

use crate::db::models::automation::{AutomationRule, NewAutomationRule, UpdateAutomationRule};
use crate::error::AppError;
use crate::schema::automation_rules;

pub struct AutomationRepo;

impl AutomationRepo {
    pub fn create(
        conn: &mut PgConnection,
        rule: &NewAutomationRule,
    ) -> Result<AutomationRule, AppError> {
        diesel::insert_into(automation_rules::table)
            .values(rule)
            .returning(AutomationRule::as_returning())
            .get_result(conn)
            .map_err(|e| AppError::Internal(format!("Failed to create automation rule: {}", e)))
    }

    pub fn find_by_id(
        conn: &mut PgConnection,
        rule_id: Uuid,
    ) -> Result<AutomationRule, AppError> {
        automation_rules::table
            .filter(automation_rules::id.eq(rule_id))
            .first(conn)
            .map_err(|e| AppError::NotFound { resource: format!("Automation rule {} not found", rule_id) })
    }

    pub fn list_by_workspace(
        conn: &mut PgConnection,
        workspace_id: Uuid,
    ) -> Result<Vec<AutomationRule>, AppError> {
        automation_rules::table
            .filter(automation_rules::workspace_id.eq(workspace_id))
            .order(automation_rules::created_at.desc())
            .load(conn)
            .map_err(|e| AppError::Internal(format!("Failed to list automation rules: {}", e)))
    }

    pub fn list_by_trigger(
        conn: &mut PgConnection,
        workspace_id: Uuid,
        trigger_type: &str,
    ) -> Result<Vec<AutomationRule>, AppError> {
        automation_rules::table
            .filter(automation_rules::workspace_id.eq(workspace_id))
            .filter(automation_rules::is_enabled.eq(true))
            .filter(automation_rules::trigger_type.eq(trigger_type))
            .load(conn)
            .map_err(|e| AppError::Internal(format!("Failed to list rules by trigger: {}", e)))
    }

    pub fn update(
        conn: &mut PgConnection,
        rule_id: Uuid,
        updates: &UpdateAutomationRule,
    ) -> Result<AutomationRule, AppError> {
        diesel::update(automation_rules::table.filter(automation_rules::id.eq(rule_id)))
            .set(updates)
            .returning(AutomationRule::as_returning())
            .get_result(conn)
            .map_err(|e| AppError::Internal(format!("Failed to update automation rule: {}", e)))
    }

    pub fn delete(conn: &mut PgConnection, rule_id: Uuid) -> Result<(), AppError> {
        let deleted = diesel::delete(automation_rules::table.filter(automation_rules::id.eq(rule_id)))
            .execute(conn)
            .map_err(|e| AppError::Internal(format!("Failed to delete automation rule: {}", e)))?;
        if deleted == 0 {
            return Err(AppError::NotFound { resource: format!("Automation rule {} not found", rule_id) });
        }
        Ok(())
    }
}