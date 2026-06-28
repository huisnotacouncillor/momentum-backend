#[cfg(test)]
mod tests {
    use momentum_core::db::models::automation::{Condition, TriggerType};
    use momentum_core::db::models::issue::Issue;
    use momentum_core::services::automation_engine::AutomationEngine;
    use uuid::Uuid;

    // 辅助函数：创建测试用的 Issue
    fn create_test_issue(priority: &str, assignee_id: Option<Uuid>) -> Issue {
        Issue {
            id: Uuid::new_v4(),
            team_id: Uuid::new_v4(),
            project_id: Some(Uuid::new_v4()),
            cycle_id: None,
            creator_id: Uuid::new_v4(),
            assignee_id,
            parent_issue_id: None,
            issue_number: 1,
            title: "Test Issue".to_string(),
            description: None,
            priority: priority.to_string(),
            workflow_state_id: Some(Uuid::new_v4()),
            workflow_id: Some(Uuid::new_v4()),
            is_changelog_candidate: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
        }
    }

    #[test]
    fn test_evaluate_condition_equals_matching() {
        let issue = create_test_issue("high", None);
        let condition = Condition {
            field: "priority".to_string(),
            operator: "equals".to_string(),
            value: serde_json::json!("high"),
        };

        let result = AutomationEngine::evaluate_condition(&condition, &issue);
        assert!(result, "Expected condition to match");
    }

    #[test]
    fn test_evaluate_condition_equals_not_matching() {
        let issue = create_test_issue("high", None);
        let condition = Condition {
            field: "priority".to_string(),
            operator: "equals".to_string(),
            value: serde_json::json!("low"),
        };

        let result = AutomationEngine::evaluate_condition(&condition, &issue);
        assert!(!result, "Expected condition to NOT match");
    }

    #[test]
    fn test_evaluate_condition_not_equals() {
        let issue = create_test_issue("high", None);
        let condition = Condition {
            field: "priority".to_string(),
            operator: "not_equals".to_string(),
            value: serde_json::json!("low"),
        };

        let result = AutomationEngine::evaluate_condition(&condition, &issue);
        assert!(result, "Expected not_equals to match");
    }

    #[test]
    fn test_evaluate_condition_is_null_matching() {
        let issue = create_test_issue("high", None);
        let condition = Condition {
            field: "assignee_id".to_string(),
            operator: "is_null".to_string(),
            value: serde_json::json!(null),
        };

        let result = AutomationEngine::evaluate_condition(&condition, &issue);
        assert!(result, "Expected is_null to match when assignee_id is None");
    }

    #[test]
    fn test_evaluate_condition_is_not_null_matching() {
        let assignee_id = Uuid::new_v4();
        let issue = create_test_issue("high", Some(assignee_id));
        let condition = Condition {
            field: "assignee_id".to_string(),
            operator: "is_not_null".to_string(),
            value: serde_json::json!(null),
        };

        let result = AutomationEngine::evaluate_condition(&condition, &issue);
        assert!(result, "Expected is_not_null to match when assignee_id is Some");
    }

    #[test]
    fn test_evaluate_conditions_and_all_match() {
        let assignee_id = Uuid::new_v4();
        let issue = create_test_issue("high", Some(assignee_id));
        let conditions = vec![
            Condition {
                field: "priority".to_string(),
                operator: "equals".to_string(),
                value: serde_json::json!("high"),
            },
            Condition {
                field: "assignee_id".to_string(),
                operator: "is_not_null".to_string(),
                value: serde_json::json!(null),
            },
        ];

        let result = AutomationEngine::evaluate_conditions(&conditions, &issue, "and");
        assert!(result, "Expected all conditions to match with AND");
    }

    #[test]
    fn test_evaluate_conditions_and_one_fails() {
        let issue = create_test_issue("high", None);
        let conditions = vec![
            Condition {
                field: "priority".to_string(),
                operator: "equals".to_string(),
                value: serde_json::json!("high"),
            },
            Condition {
                field: "assignee_id".to_string(),
                operator: "is_not_null".to_string(),
                value: serde_json::json!(null),
            },
        ];

        let result = AutomationEngine::evaluate_conditions(&conditions, &issue, "and");
        assert!(!result, "Expected AND to fail when one condition fails");
    }

    #[test]
    fn test_evaluate_conditions_or_one_matches() {
        let issue = create_test_issue("high", None);
        let conditions = vec![
            Condition {
                field: "priority".to_string(),
                operator: "equals".to_string(),
                value: serde_json::json!("low"),
            },
            Condition {
                field: "priority".to_string(),
                operator: "equals".to_string(),
                value: serde_json::json!("high"),
            },
        ];

        let result = AutomationEngine::evaluate_conditions(&conditions, &issue, "or");
        assert!(result, "Expected OR to match when one condition passes");
    }

    #[test]
    fn test_trigger_type_as_str() {
        assert_eq!(TriggerType::IssueCreated.as_str(), "issue_created");
        assert_eq!(TriggerType::IssueUpdated.as_str(), "issue_updated");
        assert_eq!(TriggerType::IssueStatusChanged.as_str(), "issue_status_changed");
        assert_eq!(TriggerType::IssueAssigned.as_str(), "issue_assigned");
    }

    #[test]
    fn test_trigger_type_from_str() {
        assert_eq!(TriggerType::from_str("issue_created"), Some(TriggerType::IssueCreated));
        assert_eq!(TriggerType::from_str("issue_updated"), Some(TriggerType::IssueUpdated));
        assert_eq!(TriggerType::from_str("issue_status_changed"), Some(TriggerType::IssueStatusChanged));
        assert_eq!(TriggerType::from_str("issue_assigned"), Some(TriggerType::IssueAssigned));
        assert_eq!(TriggerType::from_str("invalid"), None);
    }
}
