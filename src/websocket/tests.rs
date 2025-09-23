#[cfg(test)]
mod tests {
    // use super::*;
    use crate::websocket::{
        WebSocketCommand, WebSocketCommandResponse,
        WebSocketRateLimiter, RateLimitConfig, WebSocketErrorHandler,
        RetryTimeoutManager, RetryConfig, TimeoutConfig,
        WebSocketErrorMapper, WebSocketErrorCode,
    };
    // use crate::db::models::ErrorDetail;
    use crate::db::enums::LabelLevel;
    // use crate::services::context::RequestContext;
    use crate::error::AppError;
    use uuid::Uuid;
    // use std::sync::Arc;

    /// 测试WebSocket命令序列化和反序列化
    #[test]
    fn test_websocket_command_serialization() {
        let command = WebSocketCommand::CreateLabel {
            idempotency_key: "test-key-123".to_string(),
            data: crate::websocket::commands::CreateLabelCommand {
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
            WebSocketCommand::CreateLabel { idempotency_key, data } => {
                assert_eq!(idempotency_key, "test-key-123");
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
        let response = WebSocketCommandResponse {
            idempotency_key: "test-key-123".to_string(),
            success: true,
            data: Some(serde_json::json!({
                "id": "label-123",
                "name": "Test Label",
                "color": "#FF0000"
            })),
            error: None,
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: WebSocketCommandResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.idempotency_key, "test-key-123");
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
            assert!(!limiter.is_rate_limited(user_id, None).await, "Request {} should pass", i + 1);
        }

        // 第4个请求应该被限流
        assert!(limiter.is_rate_limited(user_id, None).await, "Request 4 should be rate limited");
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
            assert!(mapper.should_retry(&error), "Error {:?} should be retryable", error_code);
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
            assert!(!mapper.should_retry(&error), "Error {:?} should not be retryable", error_code);
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
            assert!(mapper.should_disconnect(&error), "Error {:?} should trigger disconnect", error_code);
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
            assert!(!mapper.should_disconnect(&error), "Error {:?} should not trigger disconnect", error_code);
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

        let result = manager.execute_with_retry(
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
        ).await;

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

        let result = manager.execute_with_retry(
            || Box::pin(async { Err::<String, _>("Always fails") }),
            "test_operation",
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::websocket::retry_timeout::RetryTimeoutError::MaxRetriesExceeded(_) => {},
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

        let result = manager.execute_with_retry(
            || {
                Box::pin(async {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    Ok::<String, String>("Should not reach here".to_string())
                })
            },
            "test_operation",
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::websocket::retry_timeout::RetryTimeoutError::Timeout(_) | crate::websocket::retry_timeout::RetryTimeoutError::MaxRetriesExceeded(_) => {},
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
        assert!(!manager.should_retry(&AppError::Conflict { message: "Conflict".into(), field: None, code: None }));
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
        use crate::websocket::manager::{WebSocketMessage, MessageType};

        let message = WebSocketMessage {
            id: "test-id".to_string(),
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
            timestamp: chrono::Utc::now(),
            from_user_id: Some(Uuid::new_v4()),
            to_user_id: Some(Uuid::new_v4()),
            secure_message: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "test-id");
        assert_eq!(deserialized.message_type, MessageType::Command);
        assert!(deserialized.from_user_id.is_some());
        assert!(deserialized.to_user_id.is_some());
    }
}
