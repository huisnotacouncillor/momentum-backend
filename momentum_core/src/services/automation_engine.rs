use diesel::prelude::*;
use diesel::ExpressionMethods;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::models::automation::{Action, Condition, TriggerType};
use crate::db::models::issue::{Issue, NewIssueLabel};
use crate::db::repositories::automation::AutomationRepo;
use crate::db::DbPool;
use crate::error::AppError;

pub struct AutomationEngine {
    pool: Arc<DbPool>,
}

impl AutomationEngine {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    /// 评估条件是否匹配
    pub fn evaluate_condition(condition: &Condition, issue: &Issue) -> bool {
        let field_value = match condition.field.as_str() {
            "priority" => serde_json::json!(issue.priority.clone()),
            "assignee_id" => serde_json::json!(issue.assignee_id),
            "team_id" => serde_json::json!(issue.team_id),
            "project_id" => serde_json::json!(issue.project_id),
            "cycle_id" => serde_json::json!(issue.cycle_id),
            _ => return false,
        };

        match condition.operator.as_str() {
            "equals" => field_value == condition.value,
            "not_equals" => field_value != condition.value,
            "is_null" => field_value.is_null(),
            "is_not_null" => !field_value.is_null(),
            _ => false,
        }
    }

    /// 评估条件组（AND/OR）
    pub fn evaluate_conditions(conditions: &[Condition], issue: &Issue, operator: &str) -> bool {
        match operator {
            "and" => conditions.iter().all(|c| Self::evaluate_condition(c, issue)),
            "or" => conditions.iter().any(|c| Self::evaluate_condition(c, issue)),
            _ => false,
        }
    }

    /// 执行单个动作
    pub async fn execute_action(
        action: &Action,
        issue_id: Uuid,
        pool: &DbPool,
    ) -> Result<(), AppError> {
        let mut conn = pool.get().map_err(|e| AppError::internal(e.to_string()))?;

        match action {
            Action::TransitionState { state_id } => {
                use crate::schema::issues::dsl::{id, issues, workflow_state_id};
                diesel::update(issues.filter(id.eq(issue_id)))
                    .set(workflow_state_id.eq(Some(*state_id)))
                    .execute(&mut conn)
                    .map_err(|e| AppError::internal(format!("Failed to transition state: {}", e)))?;
            }
            Action::AddLabel { label_id } => {
                use crate::schema::issue_labels::dsl::issue_labels;
                let new_label = NewIssueLabel {
                    issue_id,
                    label_id: *label_id,
                };
                diesel::insert_into(issue_labels)
                    .values(&new_label)
                    .on_conflict_do_nothing()
                    .execute(&mut conn)
                    .map_err(|e| AppError::internal(format!("Failed to add label: {}", e)))?;
            }
            Action::RemoveLabel { label_id: label_uuid } => {
                use crate::schema::issue_labels::dsl::{issue_id, issue_labels, label_id};
                diesel::delete(issue_labels.filter(issue_id.eq(issue_id).and(label_id.eq(*label_uuid))))
                    .execute(&mut conn)
                    .map_err(|e| AppError::internal(format!("Failed to remove label: {}", e)))?;
            }
            Action::AssignTo { user_id } => {
                use crate::schema::issues::dsl::{assignee_id, id, issues};
                diesel::update(issues.filter(id.eq(issue_id)))
                    .set(assignee_id.eq(Some(*user_id)))
                    .execute(&mut conn)
                    .map_err(|e| AppError::internal(format!("Failed to assign: {}", e)))?;
            }
            Action::SetPriority { priority } => {
                use crate::schema::issues::dsl::{id, issues, priority};
                diesel::update(issues.filter(id.eq(issue_id)))
                    .set(priority.eq(priority.clone()))
                    .execute(&mut conn)
                    .map_err(|e| AppError::internal(format!("Failed to set priority: {}", e)))?;
            }
        }
        Ok(())
    }

    /// 处理触发器
    pub async fn handle_trigger(
        &self,
        trigger_type: TriggerType,
        issue: &Issue,
        workspace_id: Uuid,
    ) -> Result<(), AppError> {
        let mut conn = self.pool.get().map_err(|e| AppError::internal(e.to_string()))?;

        // 查询匹配的规则
        let rules = AutomationRepo::list_by_trigger(&mut conn, workspace_id, trigger_type.as_str())?;

        for rule in rules {
            // 解析条件
            let conditions: Vec<Condition> = serde_json::from_value(rule.conditions)
                .unwrap_or_default();

            // 评估条件（使用 AND 逻辑）
            if !conditions.is_empty() && !Self::evaluate_conditions(&conditions, issue, "and") {
                continue;
            }

            // 解析并执行动作
            let actions: Vec<Action> = serde_json::from_value(rule.actions)
                .unwrap_or_default();

            for action in actions {
                if let Err(e) = Self::execute_action(&action, issue.id, &self.pool).await {
                    tracing::warn!("Failed to execute automation action: {}", e);
                }
            }
        }

        Ok(())
    }
}
