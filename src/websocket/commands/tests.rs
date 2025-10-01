#[cfg(test)]
mod tests {
    use super::super::types::*;
    use crate::db::enums::LabelLevel;

    #[test]
    fn test_create_label_command_serialization() {
        let command = WebSocketCommand::CreateLabel {
            data: CreateLabelCommand {
                name: "Test Label".to_string(),
                color: "#FF0000".to_string(),
                level: LabelLevel::Project,
            },
            request_id: Some("req-123".to_string()),
        };

        let json = serde_json::to_string(&command).unwrap();
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketCommand::CreateLabel { data, request_id } => {
                assert_eq!(request_id, Some("req-123".to_string()));
                assert_eq!(data.name, "Test Label");
                assert_eq!(data.color, "#FF0000");
                assert_eq!(data.level, LabelLevel::Project);
            }
            _ => panic!("Expected CreateLabel command"),
        }
    }

    #[test]
    fn test_command_response_serialization() {
        let response = WebSocketCommandResponse::success(
            "query_labels",
            "test-key",
            Some("req-123".to_string()),
            serde_json::json!({"id": "123"}),
        );

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: WebSocketCommandResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.command_type, "query_labels");
        assert_eq!(deserialized.idempotency_key, "test-key");
        assert_eq!(deserialized.request_id, Some("req-123".to_string()));
        assert!(deserialized.success);
        assert!(deserialized.data.is_some());
        assert!(deserialized.error.is_none());
    }

    #[tokio::test]
    async fn test_idempotency_control() {
        let control = IdempotencyControl::new(60);

        let response1 = WebSocketCommandResponse::success(
            "test_command",
            "test-key",
            Some("req-123".to_string()),
            serde_json::json!({"result": "first"}),
        );

        assert!(control.is_processed("test-key").await.is_none());
        control
            .mark_processed("test-key".to_string(), response1.clone())
            .await;

        let cached = control.is_processed("test-key").await.unwrap();
        assert_eq!(cached.idempotency_key, "test-key");
        assert!(cached.success);
    }

    #[test]
    fn test_add_team_member_command_serialization() {
        let team_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        let command = WebSocketCommand::AddTeamMember {
            team_id,
            data: AddTeamMemberCommand {
                user_id,
                role: TeamMemberRole::Admin,
            },
            request_id: Some("req-add".to_string()),
        };

        let json = serde_json::to_string(&command).unwrap();
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketCommand::AddTeamMember {
                team_id: t,
                data,
                request_id,
            } => {
                assert_eq!(request_id, Some("req-add".to_string()));
                assert_eq!(t, team_id);
                assert_eq!(data.user_id, user_id);
                match data.role {
                    TeamMemberRole::Admin => (),
                    _ => panic!("expected Admin"),
                }
            }
            _ => panic!("Expected AddTeamMember command"),
        }
    }

    #[test]
    fn test_update_team_member_command_serialization() {
        let team_id = uuid::Uuid::new_v4();
        let member_user_id = uuid::Uuid::new_v4();
        let command = WebSocketCommand::UpdateTeamMember {
            team_id,
            member_user_id,
            data: UpdateTeamMemberCommand {
                role: TeamMemberRole::Member,
            },
            request_id: Some("req-upd".to_string()),
        };

        let json = serde_json::to_string(&command).unwrap();
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketCommand::UpdateTeamMember {
                team_id: t,
                member_user_id: u,
                data,
                request_id,
            } => {
                assert_eq!(request_id, Some("req-upd".to_string()));
                assert_eq!(t, team_id);
                assert_eq!(u, member_user_id);
                match data.role {
                    TeamMemberRole::Member => (),
                    _ => panic!("expected Member"),
                }
            }
            _ => panic!("Expected UpdateTeamMember command"),
        }
    }

    #[test]
    fn test_remove_team_member_command_serialization() {
        let team_id = uuid::Uuid::new_v4();
        let member_user_id = uuid::Uuid::new_v4();
        let command = WebSocketCommand::RemoveTeamMember {
            team_id,
            member_user_id,
            request_id: Some("req-del".to_string()),
        };

        let json = serde_json::to_string(&command).unwrap();
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketCommand::RemoveTeamMember {
                team_id: t,
                member_user_id: u,
                request_id,
            } => {
                assert_eq!(request_id, Some("req-del".to_string()));
                assert_eq!(t, team_id);
                assert_eq!(u, member_user_id);
            }
            _ => panic!("Expected RemoveTeamMember command"),
        }
    }

    #[test]
    fn test_list_team_members_command_serialization() {
        let team_id = uuid::Uuid::new_v4();
        let command = WebSocketCommand::ListTeamMembers {
            team_id,
            request_id: Some("req-list".to_string()),
        };

        let json = serde_json::to_string(&command).unwrap();
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketCommand::ListTeamMembers {
                team_id: t,
                request_id,
            } => {
                assert_eq!(request_id, Some("req-list".to_string()));
                assert_eq!(t, team_id);
            }
            _ => panic!("Expected ListTeamMembers command"),
        }
    }

    #[test]
    fn test_query_workspace_members_command_serialization() {
        let command = WebSocketCommand::QueryWorkspaceMembers {
            filters: WorkspaceMemberFilters {
                role: Some(WorkspaceMemberRole::Admin),
                user_id: None,
                search: Some("john".to_string()),
            },
            request_id: Some("req-query-ws".to_string()),
        };

        let json = serde_json::to_string(&command).unwrap();
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketCommand::QueryWorkspaceMembers { filters, request_id } => {
                assert_eq!(request_id, Some("req-query-ws".to_string()));
                assert!(matches!(filters.role, Some(WorkspaceMemberRole::Admin)));
                assert_eq!(filters.user_id, None);
                assert_eq!(filters.search, Some("john".to_string()));
            }
            _ => panic!("Expected QueryWorkspaceMembers command"),
        }
    }

    #[test]
    fn test_query_workspace_members_without_search() {
        let command = WebSocketCommand::QueryWorkspaceMembers {
            filters: WorkspaceMemberFilters {
                role: None,
                user_id: None,
                search: None,
            },
            request_id: Some("req-query-ws-no-search".to_string()),
        };

        let json = serde_json::to_string(&command).unwrap();
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketCommand::QueryWorkspaceMembers { filters, request_id } => {
                assert_eq!(request_id, Some("req-query-ws-no-search".to_string()));
                assert_eq!(filters.role, None);
                assert_eq!(filters.user_id, None);
                assert_eq!(filters.search, None);
            }
            _ => panic!("Expected QueryWorkspaceMembers command"),
        }
    }
}
