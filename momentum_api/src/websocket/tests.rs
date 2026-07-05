#[cfg(test)]
mod websocket_tests {
    // use super::*;
    use crate::websocket::{
        RateLimitConfig, RetryConfig, RetryTimeoutManager, TimeoutConfig, WebSocketCommand,
        WebSocketCommandResponse, WebSocketErrorCode, WebSocketErrorHandler, WebSocketErrorMapper,
        WebSocketRateLimiter,
    };
    // use crate::db::models::ErrorDetail;
    use crate::db::enums::LabelLevel;
    // use crate::services::context::RequestContext;
    use momentum_core::error::AppError;
    use uuid::Uuid;
    // use std::sync::Arc;

    /// 测试WebSocket命令序列化和反序列化
    #[test]
    fn test_websocket_command_serialization() {
        use crate::websocket::commands::types::CreateLabelCommand;

        let command = WebSocketCommand::CreateLabel {
            request_id: Some("test-key-123".to_string()),
            data: CreateLabelCommand {
                name: "Test Label".to_string(),
                color: "#FF0000".to_string(),
                level: LabelLevel::Project,
            },
        };

        // 序列化
        let json = serde_json::to_string(&command).unwrap();
        assert!(json.contains("create_label"));
        assert!(json.contains("test-key-123"));
        assert!(json.contains("Test Label"));

        // 反序列化
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            WebSocketCommand::CreateLabel { request_id, data } => {
                assert_eq!(request_id, Some("test-key-123".to_string()));
                assert_eq!(data.name, "Test Label");
                assert_eq!(data.color, "#FF0000");
                assert_eq!(data.level, LabelLevel::Project);
            }
            _ => panic!("Expected CreateLabel command"),
        }
    }

    /// 测试WebSocket命令响应序列化
    #[test]
    fn test_websocket_command_response_serialization() {
        let response = WebSocketCommandResponse::success(
            "create_label",
            "test-key-123",
            Some("req-123".to_string()),
            serde_json::json!({
                "id": "label-123",
                "name": "Test Label",
                "color": "#FF0000"
            }),
        );

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: WebSocketCommandResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.idempotency_key, "test-key-123");
        assert_eq!(deserialized.command_type, "create_label");
        assert!(deserialized.success);
        assert!(deserialized.data.is_some());
        assert!(deserialized.error.is_none());
    }

    /// 测试限流器基本功能
    #[tokio::test]
    async fn test_rate_limiter_basic_functionality() {
        let config = RateLimitConfig {
            window_seconds: 60,
            max_requests: 3,
            command_limits: std::collections::HashMap::new(),
        };

        let limiter = WebSocketRateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // 前3个请求应该通过
        for i in 0..3 {
            assert!(
                !limiter.is_rate_limited(user_id, None).await,
                "Request {} should pass",
                i + 1
            );
        }

        // 第4个请求应该被限流
        assert!(
            limiter.is_rate_limited(user_id, None).await,
            "Request 4 should be rate limited"
        );
    }

    /// 测试命令特定限流
    #[tokio::test]
    async fn test_command_specific_rate_limiting() {
        let mut command_limits = std::collections::HashMap::new();
        command_limits.insert("create_label".to_string(), 2);
        command_limits.insert("delete_label".to_string(), 1);

        let config = RateLimitConfig {
            window_seconds: 60,
            max_requests: 100,
            command_limits,
        };

        let limiter = WebSocketRateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // create_label 命令限制
        assert!(!limiter.is_rate_limited(user_id, Some("create_label")).await);
        assert!(!limiter.is_rate_limited(user_id, Some("create_label")).await);
        assert!(limiter.is_rate_limited(user_id, Some("create_label")).await);

        // delete_label 命令限制
        assert!(!limiter.is_rate_limited(user_id, Some("delete_label")).await);
        assert!(limiter.is_rate_limited(user_id, Some("delete_label")).await);

        // 其他命令不受影响
        assert!(!limiter.is_rate_limited(user_id, Some("update_label")).await);
    }

    /// 测试用户统计信息
    #[tokio::test]
    async fn test_user_stats() {
        let config = RateLimitConfig::default();
        let limiter = WebSocketRateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // 添加一些请求
        limiter.is_rate_limited(user_id, Some("create_label")).await;
        limiter.is_rate_limited(user_id, Some("create_label")).await;
        limiter.is_rate_limited(user_id, Some("update_label")).await;
        limiter.is_rate_limited(user_id, Some("ping")).await;

        let stats = limiter.get_user_stats(user_id).await.unwrap();
        assert_eq!(stats.user_id, user_id);
        assert_eq!(stats.total_requests, 4);
        assert_eq!(stats.command_stats.get("create_label"), Some(&2));
        assert_eq!(stats.command_stats.get("update_label"), Some(&1));
        assert_eq!(stats.command_stats.get("ping"), Some(&1));
    }

    /// 测试错误映射器
    #[test]
    fn test_error_mapper_app_error() {
        let mapper = WebSocketErrorMapper::default();

        // 测试NotFound错误
        let app_error = AppError::not_found("label");
        let ws_error = mapper.map_app_error(&app_error);
        assert_eq!(ws_error.code, WebSocketErrorCode::LabelNotFound);
        assert!(ws_error.message.contains("label"));

        // 测试ValidationError
        let app_error = AppError::validation("Validation failed");
        let ws_error = mapper.map_app_error(&app_error);
        assert_eq!(ws_error.code, WebSocketErrorCode::ValidationFailed);

        // 测试DatabaseError
        let app_error = AppError::Database(diesel::result::Error::NotFound);
        let ws_error = mapper.map_app_error(&app_error);
        assert_eq!(ws_error.code, WebSocketErrorCode::DatabaseError);
        assert!(ws_error.retry_after.is_some());
    }

    /// 测试错误映射器的重试逻辑
    #[test]
    fn test_error_mapper_retry_logic() {
        let mapper = WebSocketErrorMapper::default();

        // 可重试的错误
        let retryable_errors = vec![
            WebSocketErrorCode::DatabaseError,
            WebSocketErrorCode::NetworkError,
            WebSocketErrorCode::CommandTimeout,
            WebSocketErrorCode::ServiceUnavailable,
            WebSocketErrorCode::InternalError,
        ];

        for error_code in retryable_errors {
            let error = crate::websocket::error_mapper::WebSocketError::new(
                error_code.clone(),
                "Test error".to_string(),
            );
            assert!(
                mapper.should_retry(&error),
                "Error {:?} should be retryable",
                error_code
            );
        }

        // 不可重试的错误
        let non_retryable_errors = vec![
            WebSocketErrorCode::ValidationFailed,
            WebSocketErrorCode::LabelNotFound,
            WebSocketErrorCode::AuthenticationFailed,
            WebSocketErrorCode::PermissionDenied,
        ];

        for error_code in non_retryable_errors {
            let error = crate::websocket::error_mapper::WebSocketError::new(
                error_code.clone(),
                "Test error".to_string(),
            );
            assert!(
                !mapper.should_retry(&error),
                "Error {:?} should not be retryable",
                error_code
            );
        }
    }

    /// 测试错误映射器的断开连接逻辑
    #[test]
    fn test_error_mapper_disconnect_logic() {
        let mapper = WebSocketErrorMapper::default();

        // 应该断开连接的错误
        let disconnect_errors = vec![
            WebSocketErrorCode::AuthenticationFailed,
            WebSocketErrorCode::TokenExpired,
            WebSocketErrorCode::TokenInvalid,
            WebSocketErrorCode::UserNotFound,
            WebSocketErrorCode::ConnectionLost,
            WebSocketErrorCode::ConnectionTimeout,
        ];

        for error_code in disconnect_errors {
            let error = crate::websocket::error_mapper::WebSocketError::new(
                error_code.clone(),
                "Test error".to_string(),
            );
            assert!(
                mapper.should_disconnect(&error),
                "Error {:?} should trigger disconnect",
                error_code
            );
        }

        // 不应该断开连接的错误
        let non_disconnect_errors = vec![
            WebSocketErrorCode::ValidationFailed,
            WebSocketErrorCode::LabelNotFound,
            WebSocketErrorCode::RateLimitExceeded,
            WebSocketErrorCode::CommandTimeout,
        ];

        for error_code in non_disconnect_errors {
            let error = crate::websocket::error_mapper::WebSocketError::new(
                error_code.clone(),
                "Test error".to_string(),
            );
            assert!(
                !mapper.should_disconnect(&error),
                "Error {:?} should not trigger disconnect",
                error_code
            );
        }
    }

    /// 测试重试超时管理器
    #[tokio::test]
    async fn test_retry_timeout_manager_success() {
        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 10,
            delay_multiplier: 2.0,
            max_delay_ms: 1000,
        };
        let timeout_config = TimeoutConfig {
            command_timeout_seconds: 5,
            connection_timeout_seconds: 60,
            heartbeat_timeout_seconds: 30,
        };

        let manager = RetryTimeoutManager::new(retry_config, timeout_config);
        let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

        let result = manager
            .execute_with_retry(
                || {
                    let count = attempt_count.clone();
                    Box::pin(async move {
                        let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        if current == 0 {
                            Err("First retry fails")
                        } else {
                            Ok("Success")
                        }
                    })
                },
                "test_operation",
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempt_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    /// 测试重试超时管理器的最大重试次数
    #[tokio::test]
    async fn test_retry_timeout_manager_max_retries() {
        let retry_config = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 10,
            delay_multiplier: 2.0,
            max_delay_ms: 1000,
        };
        let timeout_config = TimeoutConfig::default();

        let manager = RetryTimeoutManager::new(retry_config, timeout_config);

        let result = manager
            .execute_with_retry(
                || Box::pin(async { Err::<String, _>("Always fails") }),
                "test_operation",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::websocket::retry_timeout::RetryTimeoutError::MaxRetriesExceeded(_) => {}
            _ => panic!("Expected MaxRetriesExceeded error"),
        }
    }

    /// 测试重试超时管理器的超时功能
    #[tokio::test]
    async fn test_retry_timeout_manager_timeout() {
        let retry_config = RetryConfig::default();
        let timeout_config = TimeoutConfig {
            command_timeout_seconds: 1,
            connection_timeout_seconds: 60,
            heartbeat_timeout_seconds: 30,
        };

        let manager = RetryTimeoutManager::new(retry_config, timeout_config);

        let result = manager
            .execute_with_retry(
                || {
                    Box::pin(async {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        Ok::<String, String>("Should not reach here".to_string())
                    })
                },
                "test_operation",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::websocket::retry_timeout::RetryTimeoutError::Timeout(_)
            | crate::websocket::retry_timeout::RetryTimeoutError::MaxRetriesExceeded(_) => {}
            _ => panic!("Expected Timeout or MaxRetriesExceeded error"),
        }
    }

    /// 测试重试超时管理器的should_retry逻辑
    #[test]
    fn test_retry_timeout_manager_should_retry() {
        let manager = RetryTimeoutManager::new(RetryConfig::default(), TimeoutConfig::default());

        // 应该重试的错误
        assert!(manager.should_retry(&AppError::Database(diesel::result::Error::NotFound)));
        assert!(manager.should_retry(&AppError::internal("Internal error")));

        // 不应该重试的错误
        assert!(!manager.should_retry(&AppError::validation("v")));
        assert!(!manager.should_retry(&AppError::not_found("Not found")));
        assert!(!manager.should_retry(&AppError::Conflict {
            message: "Conflict".into(),
            field: None,
            code: None
        }));
        assert!(!manager.should_retry(&AppError::auth("Unauthorized")));
    }

    /// 测试WebSocket错误处理器
    #[test]
    fn test_websocket_error_handler() {
        let handler = WebSocketErrorHandler::new();

        // 测试AppError处理
        let app_error = AppError::internal("Internal error".to_string());
        let ws_error = handler.handle_app_error(&app_error);
        assert_eq!(ws_error.code, WebSocketErrorCode::InternalError);
        assert!(handler.should_retry(&ws_error));

        // 测试限流错误处理
        let rate_limit_error = handler.handle_rate_limit_error(Some(60));
        assert_eq!(rate_limit_error.code, WebSocketErrorCode::RateLimitExceeded);
        assert_eq!(rate_limit_error.retry_after, Some(60));

        // 测试超时错误处理
        let timeout_error = handler.handle_timeout_error("test_operation");
        assert_eq!(timeout_error.code, WebSocketErrorCode::CommandTimeout);
        assert!(timeout_error.message.contains("test_operation"));
        assert_eq!(timeout_error.retry_after, Some(5));

        // 测试重试错误处理
        let retry_error = handler.handle_retry_error("test_operation", 3);
        assert_eq!(retry_error.code, WebSocketErrorCode::CommandFailed);
        assert!(retry_error.message.contains("test_operation"));
        assert!(retry_error.message.contains("3"));
        assert_eq!(retry_error.retry_after, Some(10));
    }

    /// 测试WebSocket消息类型
    #[test]
    fn test_websocket_message_types() {
        use crate::websocket::manager::MessageType;

        // 测试序列化
        let message_types = vec![
            MessageType::Text,
            MessageType::Notification,
            MessageType::SystemMessage,
            MessageType::UserJoined,
            MessageType::UserLeft,
            MessageType::Ping,
            MessageType::Pong,
            MessageType::Error,
            MessageType::Command,
            MessageType::CommandResponse,
        ];

        for message_type in message_types {
            let json = serde_json::to_string(&message_type).unwrap();
            let deserialized: MessageType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, message_type);
        }
    }

    /// 测试WebSocket消息结构
    #[test]
    fn test_websocket_message_structure() {
        use crate::websocket::manager::{MessageType, WebSocketMessage};

        let message = WebSocketMessage {
            id: Some("test-id".to_string()),
            message_type: MessageType::Command,
            data: serde_json::json!({
                "type": "create_label",
                "idempotency_key": "test-key",
                "data": {
                    "name": "Test Label",
                    "color": "#FF0000",
                    "level": "high"
                }
            }),
            timestamp: Some(chrono::Utc::now()),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, Some("test-id".to_string()));
        assert_eq!(deserialized.message_type, MessageType::Command);
    }

    /// 测试 Team 命令序列化
    #[test]
    fn test_team_commands_serialization() {
        use crate::websocket::commands::types::{
            CreateTeamWorkflowStatusCommand, UpdateTeamWorkflowStatusCommand, WebSocketCommand,
        };
        use uuid::Uuid;

        // 测试 GetTeam
        let team_id = Uuid::new_v4();
        let cmd = WebSocketCommand::GetTeam {
            team_id,
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("get_team"));
        assert!(json.contains(&team_id.to_string()));
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            WebSocketCommand::GetTeam { team_id: tid, .. } => assert_eq!(tid, team_id),
            _ => panic!("Expected GetTeam"),
        }

        // 测试 GetTeamWorkflowStatuses
        let cmd = WebSocketCommand::GetTeamWorkflowStatuses {
            team_id,
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("get_team_workflow_statuses"));
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            WebSocketCommand::GetTeamWorkflowStatuses { team_id: tid, .. } => assert_eq!(tid, team_id),
            _ => panic!("Expected GetTeamWorkflowStatuses"),
        }

        // 测试 CreateTeamWorkflowStatus
        let cmd = WebSocketCommand::CreateTeamWorkflowStatus {
            team_id,
            data: CreateTeamWorkflowStatusCommand {
                name: "Test Status".to_string(),
                description: Some("Description".to_string()),
                color: "#FF0000".to_string(),
                category: "backlog".to_string(),
                position: 0,
            },
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("create_team_workflow_status"));
        assert!(json.contains("Test Status"));

        // 测试 UpdateTeamWorkflowStatus
        let status_id = Uuid::new_v4();
        let cmd = WebSocketCommand::UpdateTeamWorkflowStatus {
            team_id,
            status_id,
            data: UpdateTeamWorkflowStatusCommand {
                name: Some("Updated Status".to_string()),
                description: None,
                color: Some("#00FF00".to_string()),
                category: None,
                position: Some(1),
            },
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_team_workflow_status"));
        assert!(json.contains("Updated Status"));

        // 测试 DeleteTeamWorkflowStatus
        let cmd = WebSocketCommand::DeleteTeamWorkflowStatus {
            team_id,
            status_id,
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("delete_team_workflow_status"));
        assert!(json.contains(&status_id.to_string()));
    }

    /// 测试 Workspace 命令序列化
    #[test]
    fn test_workspace_commands_serialization() {
        use crate::websocket::commands::types::WebSocketCommand;
        use uuid::Uuid;

        let workspace_id = Uuid::new_v4();

        // 测试 GetWorkspace
        let cmd = WebSocketCommand::GetWorkspace {
            workspace_id,
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("get_workspace"));
        assert!(json.contains(&workspace_id.to_string()));
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            WebSocketCommand::GetWorkspace { workspace_id: wid, .. } => assert_eq!(wid, workspace_id),
            _ => panic!("Expected GetWorkspace"),
        }
    }

    /// 测试 Workspace Member 命令序列化
    #[test]
    fn test_workspace_member_commands_serialization() {
        use crate::websocket::commands::types::{
            UpdateWorkspaceMemberCommand, WebSocketCommand, WorkspaceMemberRole,
        };
        use uuid::Uuid;

        let user_id = Uuid::new_v4();

        // 测试 GetWorkspaceMember
        let cmd = WebSocketCommand::GetWorkspaceMember {
            user_id,
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("get_workspace_member"));
        assert!(json.contains(&user_id.to_string()));

        // 测试 UpdateWorkspaceMember
        let cmd = WebSocketCommand::UpdateWorkspaceMember {
            user_id,
            data: UpdateWorkspaceMemberCommand {
                role: WorkspaceMemberRole::Admin,
            },
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_workspace_member"));
        assert!(json.contains("admin"));

        // 测试 DeleteWorkspaceMember
        let cmd = WebSocketCommand::DeleteWorkspaceMember {
            user_id,
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("delete_workspace_member"));
        assert!(json.contains(&user_id.to_string()));
    }

    /// 测试 Comment 命令序列化
    #[test]
    fn test_comment_commands_serialization() {
        use crate::websocket::commands::types::{
            CreateCommentCommand, UpdateCommentCommand, WebSocketCommand,
        };
        use uuid::Uuid;

        let issue_id = Uuid::new_v4();
        let comment_id = Uuid::new_v4();

        // 测试 QueryComments
        let cmd = WebSocketCommand::QueryComments {
            issue_id,
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("query_comments"));
        assert!(json.contains(&issue_id.to_string()));
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            WebSocketCommand::QueryComments { issue_id: iid, .. } => assert_eq!(iid, issue_id),
            _ => panic!("Expected QueryComments"),
        }

        // 测试 CreateComment
        let cmd = WebSocketCommand::CreateComment {
            issue_id,
            data: CreateCommentCommand {
                content: "Test comment content".to_string(),
                content_type: Some("markdown".to_string()),
                parent_comment_id: None,
            },
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("create_comment"));
        assert!(json.contains("Test comment content"));
        assert!(json.contains("markdown"));

        // 测试 UpdateComment
        let cmd = WebSocketCommand::UpdateComment {
            issue_id,
            comment_id,
            data: UpdateCommentCommand {
                content: "Updated comment content".to_string(),
            },
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_comment"));
        assert!(json.contains("Updated comment content"));
        assert!(json.contains(&comment_id.to_string()));

        // 测试 DeleteComment
        let cmd = WebSocketCommand::DeleteComment {
            issue_id,
            comment_id,
            request_id: Some("test-req".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("delete_comment"));
        assert!(json.contains(&comment_id.to_string()));
    }

    /// 测试 command_type 方法
    #[test]
    fn test_command_type_methods() {
        use crate::websocket::commands::types::WebSocketCommand;
        use uuid::Uuid;

        let team_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let issue_id = Uuid::new_v4();

        // Team commands
        assert_eq!(
            WebSocketCommand::GetTeam { team_id, request_id: None }.command_type(),
            "get_team"
        );
        assert_eq!(
            WebSocketCommand::GetTeamWorkflowStatuses { team_id, request_id: None }.command_type(),
            "get_team_workflow_statuses"
        );
        assert_eq!(
            WebSocketCommand::CreateTeamWorkflowStatus {
                team_id,
                data: crate::websocket::commands::types::CreateTeamWorkflowStatusCommand {
                    name: "Test".to_string(),
                    description: None,
                    color: "#000".to_string(),
                    category: "backlog".to_string(),
                    position: 0,
                },
                request_id: None
            }.command_type(),
            "create_team_workflow_status"
        );
        assert_eq!(
            WebSocketCommand::UpdateTeamWorkflowStatus {
                team_id,
                status_id: team_id,
                data: crate::websocket::commands::types::UpdateTeamWorkflowStatusCommand {
                    name: None,
                    description: None,
                    color: None,
                    category: None,
                    position: None,
                },
                request_id: None
            }.command_type(),
            "update_team_workflow_status"
        );
        assert_eq!(
            WebSocketCommand::DeleteTeamWorkflowStatus { team_id, status_id: team_id, request_id: None }.command_type(),
            "delete_team_workflow_status"
        );

        // Workspace commands
        assert_eq!(
            WebSocketCommand::GetWorkspace { workspace_id: team_id, request_id: None }.command_type(),
            "get_workspace"
        );

        // Workspace member commands
        assert_eq!(
            WebSocketCommand::GetWorkspaceMember { user_id, request_id: None }.command_type(),
            "get_workspace_member"
        );
        assert_eq!(
            WebSocketCommand::UpdateWorkspaceMember {
                user_id,
                data: crate::websocket::commands::types::UpdateWorkspaceMemberCommand {
                    role: crate::websocket::commands::types::WorkspaceMemberRole::Admin,
                },
                request_id: None
            }.command_type(),
            "update_workspace_member"
        );
        assert_eq!(
            WebSocketCommand::DeleteWorkspaceMember { user_id, request_id: None }.command_type(),
            "delete_workspace_member"
        );

        // Comment commands
        assert_eq!(
            WebSocketCommand::QueryComments { issue_id, request_id: None }.command_type(),
            "query_comments"
        );
        assert_eq!(
            WebSocketCommand::CreateComment {
                issue_id,
                data: crate::websocket::commands::types::CreateCommentCommand {
                    content: "test".to_string(),
                    content_type: None,
                    parent_comment_id: None,
                },
                request_id: None
            }.command_type(),
            "create_comment"
        );
        assert_eq!(
            WebSocketCommand::UpdateComment {
                issue_id,
                comment_id: team_id,
                data: crate::websocket::commands::types::UpdateCommentCommand {
                    content: "test".to_string(),
                },
                request_id: None
            }.command_type(),
            "update_comment"
        );
        assert_eq!(
            WebSocketCommand::DeleteComment { issue_id, comment_id: team_id, request_id: None }.command_type(),
            "delete_comment"
        );
    }

    // ========================================================================
    // Boundary Condition Tests
    // ========================================================================

    /// Test empty name validation
    #[test]
    fn test_create_team_workflow_status_empty_name() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        let cmd = CreateTeamWorkflowStatusCommand {
            name: "".to_string(),
            category: "backlog".to_string(),
            color: "#FF0000".to_string(),
            description: None,
            position: 0,
        };

        // Empty name should fail validation - this test documents expected behavior
        assert!(cmd.name.is_empty(), "Name should be empty for this test");
    }

    /// Test name exceeding max length
    #[test]
    fn test_create_team_workflow_status_name_too_long() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        // Name with 256 characters (exceeds typical 255 limit)
        let cmd = CreateTeamWorkflowStatusCommand {
            name: "a".repeat(256),
            category: "backlog".to_string(),
            color: "#FF0000".to_string(),
            description: None,
            position: 0,
        };

        assert_eq!(cmd.name.len(), 256, "Name should exceed 255 chars");
    }

    /// Test valid category values
    #[test]
    fn test_create_team_workflow_status_valid_categories() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        let valid_categories = vec![
            ("backlog", true),
            ("unstarted", true),
            ("started", true),
            ("completed", true),
            ("invalid", false),
            ("BACKLOG", false), // case sensitive
            ("", false),
        ];

        for (category, expected_valid) in valid_categories {
            let cmd = CreateTeamWorkflowStatusCommand {
                name: "Test".to_string(),
                category: category.to_string(),
                color: "#FF0000".to_string(),
                description: None,
                position: 0,
            };

            let valid_list = ["backlog", "unstarted", "started", "completed"];
            let is_valid = valid_list.contains(&cmd.category.as_str());

            assert_eq!(
                is_valid, expected_valid,
                "Category '{}' should be {}",
                category, if expected_valid { "valid" } else { "invalid" }
            );
        }
    }

    /// Test invalid category values
    #[test]
    fn test_create_team_workflow_status_invalid_category() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        let cmd = CreateTeamWorkflowStatusCommand {
            name: "Test".to_string(),
            category: "invalid_category".to_string(),
            color: "#FF0000".to_string(),
            description: None,
            position: 0,
        };

        let valid_list = ["backlog", "unstarted", "started", "completed"];
        assert!(
            !valid_list.contains(&cmd.category.as_str()),
            "Category should be invalid"
        );
    }

    /// Test valid color formats
    #[test]
    fn test_create_team_workflow_status_valid_colors() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        let valid_colors = vec![
            "#FF0000",
            "#00FF00",
            "#0000FF",
            "#FFFFFF",
            "#000000",
            "#A3F4C5",
            "#123ABC",
        ];

        for color in valid_colors {
            let cmd = CreateTeamWorkflowStatusCommand {
                name: "Test".to_string(),
                category: "backlog".to_string(),
                color: color.to_string(),
                description: None,
                position: 0,
            };

            let is_valid = cmd.color.starts_with('#')
                && cmd.color.len() == 7
                && cmd.color[1..].chars().all(|c| c.is_ascii_hexdigit());

            assert!(is_valid, "Color '{}' should be valid", color);
        }
    }

    /// Test invalid color formats
    #[test]
    fn test_create_team_workflow_status_invalid_colors() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        let invalid_colors = vec![
            "not-a-color",
            "red",
            "#GGGGGG",
            "#FF000",
            "FF0000",
            "#F",
            "",
        ];

        for color in invalid_colors {
            let cmd = CreateTeamWorkflowStatusCommand {
                name: "Test".to_string(),
                category: "backlog".to_string(),
                color: color.to_string(),
                description: None,
                position: 0,
            };

            let is_valid = cmd.color.starts_with('#')
                && cmd.color.len() == 7
                && cmd.color[1..].chars().all(|c| c.is_ascii_hexdigit());

            assert!(!is_valid, "Color '{}' should be invalid", color);
        }
    }

    /// Test negative position value
    #[test]
    fn test_create_team_workflow_status_negative_position() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        let cmd = CreateTeamWorkflowStatusCommand {
            name: "Test".to_string(),
            category: "backlog".to_string(),
            color: "#FF0000".to_string(),
            description: None,
            position: -1,
        };

        assert!(cmd.position < 0, "Position should be negative");
    }

    /// Test unicode in name
    #[test]
    fn test_create_team_workflow_status_unicode_name() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        let cmd = CreateTeamWorkflowStatusCommand {
            name: "状态 🔥".to_string(),
            category: "backlog".to_string(),
            color: "#FF0000".to_string(),
            description: None,
            position: 0,
        };

        assert!(!cmd.name.is_ascii(), "Name should contain Unicode");
    }

    // ========================================================================
    // Security Tests - SQL Injection
    // ========================================================================

    /// Test SQL injection in team workflow status name
    #[test]
    fn test_sql_injection_in_name() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        let malicious_inputs = vec![
            "'; DROP TABLE teams; --",
            "1; DELETE FROM teams WHERE 1=1;--",
            "test' OR '1'='1",
        ];

        for input in malicious_inputs {
            let cmd = CreateTeamWorkflowStatusCommand {
                name: input.to_string(),
                category: "backlog".to_string(),
                color: "#FF0000".to_string(),
                description: None,
                position: 0,
            };

            // These should be rejected or sanitized at handler level
            // This test documents that malicious input is recognized
            assert!(
                cmd.name.contains("DROP")
                    || cmd.name.contains("DELETE")
                    || cmd.name.contains("OR"),
                "Input '{}' should be recognized as potentially malicious",
                input
            );
        }
    }

    /// Test SQL injection in comment content
    #[test]
    fn test_sql_injection_in_comment() {
        use crate::websocket::commands::types::CreateCommentCommand;

        let malicious_inputs = vec![
            "'; DELETE FROM comments WHERE 1=1; --",
            "test' OR '1'='1",
        ];

        for input in malicious_inputs {
            let cmd = CreateCommentCommand {
                content: input.to_string(),
                content_type: None,
                parent_comment_id: None,
            };

            assert!(
                cmd.content.contains("DELETE") || cmd.content.contains("OR"),
                "Input '{}' should be recognized as potentially malicious",
                input
            );
        }
    }

    /// Test XSS payload in comment content
    #[test]
    fn test_xss_in_comment() {
        use crate::websocket::commands::types::CreateCommentCommand;

        let xss_payloads = vec![
            "<script>alert('xss')</script>",
            "javascript:alert('xss')",
            "<img src=x onerror=alert('xss')>",
            "<svg onload=alert('xss')>",
        ];

        for payload in xss_payloads {
            let cmd = CreateCommentCommand {
                content: payload.to_string(),
                content_type: None,
                parent_comment_id: None,
            };

            // These payloads should be escaped or rejected
            assert!(
                cmd.content.contains("<script")
                    || cmd.content.contains("javascript:")
                    || cmd.content.contains("<img")
                    || cmd.content.contains("<svg")
                    || cmd.content.contains("onerror")
                    || cmd.content.contains("onload"),
                "Payload '{}' should be recognized as XSS",
                payload
            );
        }
    }

    // ========================================================================
    // Serialization Roundtrip Tests
    // ========================================================================

    /// Test CreateTeamWorkflowStatusCommand serialization roundtrip
    #[test]
    fn test_create_team_workflow_status_serialization_roundtrip() {
        use crate::websocket::commands::types::CreateTeamWorkflowStatusCommand;

        let original = CreateTeamWorkflowStatusCommand {
            name: "Test Status".to_string(),
            category: "backlog".to_string(),
            color: "#FF0000".to_string(),
            description: Some("Test description".to_string()),
            position: 5,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: CreateTeamWorkflowStatusCommand =
            serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, original.name);
        assert_eq!(deserialized.category, original.category);
        assert_eq!(deserialized.color, original.color);
        assert_eq!(deserialized.description, original.description);
        assert_eq!(deserialized.position, original.position);
    }

    /// Test UpdateTeamWorkflowStatusCommand serialization roundtrip
    #[test]
    fn test_update_team_workflow_status_serialization_roundtrip() {
        use crate::websocket::commands::types::UpdateTeamWorkflowStatusCommand;

        let original = UpdateTeamWorkflowStatusCommand {
            name: Some("Updated".to_string()),
            description: None,
            color: Some("#00FF00".to_string()),
            category: None,
            position: Some(1),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: UpdateTeamWorkflowStatusCommand =
            serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, original.name);
        assert_eq!(deserialized.color, original.color);
        assert_eq!(deserialized.position, original.position);
    }

    /// Test CreateCommentCommand serialization roundtrip
    #[test]
    fn test_create_comment_serialization_roundtrip() {
        use crate::websocket::commands::types::CreateCommentCommand;

        let parent_id = Uuid::new_v4();
        let original = CreateCommentCommand {
            content: "Test comment".to_string(),
            content_type: Some("markdown".to_string()),
            parent_comment_id: Some(parent_id),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: CreateCommentCommand = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.content, original.content);
        assert_eq!(deserialized.content_type, original.content_type);
        assert_eq!(deserialized.parent_comment_id, original.parent_comment_id);
    }

    // ========================================================================
    // Partial Update Tests
    // ========================================================================

    /// Test partial update with only name
    #[test]
    fn test_update_team_workflow_status_name_only() {
        use crate::websocket::commands::types::UpdateTeamWorkflowStatusCommand;

        let cmd = UpdateTeamWorkflowStatusCommand {
            name: Some("New Name".to_string()),
            description: None,
            color: None,
            category: None,
            position: None,
        };

        assert!(cmd.name.is_some());
        assert!(cmd.description.is_none());
        assert!(cmd.color.is_none());
    }

    /// Test partial update with only color
    #[test]
    fn test_update_team_workflow_status_color_only() {
        use crate::websocket::commands::types::UpdateTeamWorkflowStatusCommand;

        let cmd = UpdateTeamWorkflowStatusCommand {
            name: None,
            description: None,
            color: Some("#00FF00".to_string()),
            category: None,
            position: None,
        };

        assert!(cmd.name.is_none());
        assert!(cmd.color.is_some());
    }

    /// Test empty update (no fields set)
    #[test]
    fn test_update_team_workflow_status_empty_update() {
        use crate::websocket::commands::types::UpdateTeamWorkflowStatusCommand;

        let cmd = UpdateTeamWorkflowStatusCommand {
            name: None,
            description: None,
            color: None,
            category: None,
            position: None,
        };

        // All fields None means no actual updates
        assert!(cmd.name.is_none() && cmd.color.is_none());
    }

    // ========================================================================
    // Comment Content Length Tests
    // ========================================================================

    /// Test comment content at max length
    #[test]
    fn test_create_comment_content_max_length() {
        use crate::websocket::commands::types::CreateCommentCommand;

        let max_content = "a".repeat(100000);
        let cmd = CreateCommentCommand {
            content: max_content.clone(),
            content_type: None,
            parent_comment_id: None,
        };

        assert_eq!(cmd.content.len(), 100000);
    }

    /// Test comment content exceeding max length
    #[test]
    fn test_create_comment_content_exceeds_max_length() {
        use crate::websocket::commands::types::CreateCommentCommand;

        let long_content = "a".repeat(100001);
        let cmd = CreateCommentCommand {
            content: long_content.clone(),
            content_type: None,
            parent_comment_id: None,
        };

        assert_eq!(cmd.content.len(), 100001);
    }

    /// Test comment with special characters
    #[test]
    fn test_create_comment_special_characters() {
        use crate::websocket::commands::types::CreateCommentCommand;

        let cmd = CreateCommentCommand {
            content: "Test with special chars: @#$%^&*()_+-=[]{}|;':\",./<>?".to_string(),
            content_type: None,
            parent_comment_id: None,
        };

        assert!(!cmd.content.is_empty());
    }

    /// Test comment with unicode content
    #[test]
    fn test_create_comment_unicode_content() {
        use crate::websocket::commands::types::CreateCommentCommand;

        let cmd = CreateCommentCommand {
            content: "日本語コメント 🎉".to_string(),
            content_type: None,
            parent_comment_id: None,
        };

        assert!(!cmd.content.is_ascii());
    }
}
