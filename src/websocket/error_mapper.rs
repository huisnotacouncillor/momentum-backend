use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{error, warn, info};

/// 错误严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    Low,      // 一般错误，不影响核心功能
    Medium,   // 中等错误，可能影响部分功能
    High,     // 严重错误，影响核心功能
    Critical, // 致命错误，系统不可用
}

/// 错误分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ErrorCategory {
    Authentication, // 认证相关
    Authorization,  // 授权相关
    Validation,     // 验证相关
    Business,       // 业务逻辑
    System,         // 系统错误
    Network,        // 网络错误
    Database,       // 数据库错误
    RateLimit,      // 限流相关
}

/// WebSocket错误代码
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebSocketErrorCode {
    // 认证相关错误
    AuthenticationFailed,
    TokenExpired,
    TokenInvalid,
    UserNotFound,
    NoWorkspace,

    // 命令相关错误
    CommandNotFound,
    CommandInvalid,
    CommandTimeout,
    CommandFailed,

    // 限流相关错误
    RateLimitExceeded,
    TooManyRequests,

    // 幂等性相关错误
    DuplicateRequest,
    IdempotencyKeyRequired,

    // 业务逻辑错误
    LabelNotFound,
    LabelExists,
    ValidationFailed,
    PermissionDenied,

    // 系统错误
    InternalError,
    DatabaseError,
    NetworkError,
    ServiceUnavailable,

    // 连接相关错误
    ConnectionLost,
    ConnectionTimeout,
    HeartbeatTimeout,
}

/// WebSocket错误详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketError {
    pub code: WebSocketErrorCode,
    pub message: String,
    pub field: Option<String>,
    pub details: Option<serde_json::Value>,
    pub retry_after: Option<u64>,
    pub severity: ErrorSeverity,
    pub category: ErrorCategory,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub request_id: Option<String>,
}

impl WebSocketError {
    pub fn new(code: WebSocketErrorCode, message: String) -> Self {
        let (severity, category) = Self::get_error_metadata(&code);
        Self {
            code,
            message,
            field: None,
            details: None,
            retry_after: None,
            severity,
            category,
            timestamp: chrono::Utc::now(),
            request_id: None,
        }
    }

    /// 获取错误代码对应的严重程度和分类
    fn get_error_metadata(code: &WebSocketErrorCode) -> (ErrorSeverity, ErrorCategory) {
        match code {
            // 认证相关错误 - 高严重程度
            WebSocketErrorCode::AuthenticationFailed => (ErrorSeverity::High, ErrorCategory::Authentication),
            WebSocketErrorCode::TokenExpired => (ErrorSeverity::Medium, ErrorCategory::Authentication),
            WebSocketErrorCode::TokenInvalid => (ErrorSeverity::Medium, ErrorCategory::Authentication),
            WebSocketErrorCode::UserNotFound => (ErrorSeverity::Medium, ErrorCategory::Authentication),
            WebSocketErrorCode::NoWorkspace => (ErrorSeverity::Medium, ErrorCategory::Authentication),

            // 命令相关错误 - 中等严重程度
            WebSocketErrorCode::CommandNotFound => (ErrorSeverity::Medium, ErrorCategory::Validation),
            WebSocketErrorCode::CommandInvalid => (ErrorSeverity::Medium, ErrorCategory::Validation),
            WebSocketErrorCode::CommandTimeout => (ErrorSeverity::High, ErrorCategory::System),
            WebSocketErrorCode::CommandFailed => (ErrorSeverity::High, ErrorCategory::System),

            // 限流相关错误 - 低严重程度
            WebSocketErrorCode::RateLimitExceeded => (ErrorSeverity::Low, ErrorCategory::RateLimit),
            WebSocketErrorCode::TooManyRequests => (ErrorSeverity::Low, ErrorCategory::RateLimit),

            // 幂等性相关错误 - 低严重程度
            WebSocketErrorCode::DuplicateRequest => (ErrorSeverity::Low, ErrorCategory::Validation),
            WebSocketErrorCode::IdempotencyKeyRequired => (ErrorSeverity::Low, ErrorCategory::Validation),

            // 业务逻辑错误 - 中等严重程度
            WebSocketErrorCode::LabelNotFound => (ErrorSeverity::Medium, ErrorCategory::Business),
            WebSocketErrorCode::LabelExists => (ErrorSeverity::Medium, ErrorCategory::Business),
            WebSocketErrorCode::ValidationFailed => (ErrorSeverity::Medium, ErrorCategory::Validation),
            WebSocketErrorCode::PermissionDenied => (ErrorSeverity::High, ErrorCategory::Authorization),

            // 系统错误 - 高严重程度
            WebSocketErrorCode::InternalError => (ErrorSeverity::Critical, ErrorCategory::System),
            WebSocketErrorCode::DatabaseError => (ErrorSeverity::High, ErrorCategory::Database),
            WebSocketErrorCode::NetworkError => (ErrorSeverity::High, ErrorCategory::Network),
            WebSocketErrorCode::ServiceUnavailable => (ErrorSeverity::Critical, ErrorCategory::System),

            // 连接相关错误 - 高严重程度
            WebSocketErrorCode::ConnectionLost => (ErrorSeverity::High, ErrorCategory::Network),
            WebSocketErrorCode::ConnectionTimeout => (ErrorSeverity::High, ErrorCategory::Network),
            WebSocketErrorCode::HeartbeatTimeout => (ErrorSeverity::High, ErrorCategory::Network),
        }
    }

    pub fn with_field(mut self, field: String) -> Self {
        self.field = Some(field);
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// 检查是否为严重错误
    pub fn is_critical(&self) -> bool {
        matches!(self.severity, ErrorSeverity::Critical)
    }

    /// 检查是否为高优先级错误
    pub fn is_high_priority(&self) -> bool {
        matches!(self.severity, ErrorSeverity::High | ErrorSeverity::Critical)
    }

    /// 检查是否需要立即处理
    pub fn needs_immediate_attention(&self) -> bool {
        self.is_critical() || matches!(self.category, ErrorCategory::System | ErrorCategory::Database)
    }
}

/// 错误统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub errors_by_code: HashMap<WebSocketErrorCode, u64>,
    pub errors_by_severity: HashMap<ErrorSeverity, u64>,
    pub errors_by_category: HashMap<ErrorCategory, u64>,
    pub recent_errors: Vec<WebSocketError>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// 错误映射器
#[derive(Clone)]
pub struct WebSocketErrorMapper {
    /// 错误代码到用户友好消息的映射
    error_messages: HashMap<WebSocketErrorCode, String>,
    /// 错误代码到重试建议的映射
    retry_suggestions: HashMap<WebSocketErrorCode, Option<u64>>,
    /// 错误统计信息
    error_stats: Arc<RwLock<ErrorStats>>,
    /// 最大保留的错误记录数
    max_recent_errors: usize,
}

impl Default for WebSocketErrorMapper {
    fn default() -> Self {
        let mut error_messages = HashMap::new();
        error_messages.insert(WebSocketErrorCode::AuthenticationFailed, "Authentication failed".to_string());
        error_messages.insert(WebSocketErrorCode::TokenExpired, "Token has expired".to_string());
        error_messages.insert(WebSocketErrorCode::TokenInvalid, "Invalid token".to_string());
        error_messages.insert(WebSocketErrorCode::UserNotFound, "User not found".to_string());
        error_messages.insert(WebSocketErrorCode::NoWorkspace, "No workspace selected".to_string());
        error_messages.insert(WebSocketErrorCode::CommandNotFound, "Command not found".to_string());
        error_messages.insert(WebSocketErrorCode::CommandInvalid, "Invalid command format".to_string());
        error_messages.insert(WebSocketErrorCode::CommandTimeout, "Command execution timeout".to_string());
        error_messages.insert(WebSocketErrorCode::CommandFailed, "Command execution failed".to_string());
        error_messages.insert(WebSocketErrorCode::RateLimitExceeded, "Rate limit exceeded".to_string());
        error_messages.insert(WebSocketErrorCode::TooManyRequests, "Too many requests".to_string());
        error_messages.insert(WebSocketErrorCode::DuplicateRequest, "Duplicate request detected".to_string());
        error_messages.insert(WebSocketErrorCode::IdempotencyKeyRequired, "Idempotency key required".to_string());
        error_messages.insert(WebSocketErrorCode::LabelNotFound, "Label not found".to_string());
        error_messages.insert(WebSocketErrorCode::LabelExists, "Label already exists".to_string());
        error_messages.insert(WebSocketErrorCode::ValidationFailed, "Validation failed".to_string());
        error_messages.insert(WebSocketErrorCode::PermissionDenied, "Permission denied".to_string());
        error_messages.insert(WebSocketErrorCode::InternalError, "Internal server error".to_string());
        error_messages.insert(WebSocketErrorCode::DatabaseError, "Database error".to_string());
        error_messages.insert(WebSocketErrorCode::NetworkError, "Network error".to_string());
        error_messages.insert(WebSocketErrorCode::ServiceUnavailable, "Service unavailable".to_string());
        error_messages.insert(WebSocketErrorCode::ConnectionLost, "Connection lost".to_string());
        error_messages.insert(WebSocketErrorCode::ConnectionTimeout, "Connection timeout".to_string());
        error_messages.insert(WebSocketErrorCode::HeartbeatTimeout, "Heartbeat timeout".to_string());

        let mut retry_suggestions = HashMap::new();
        retry_suggestions.insert(WebSocketErrorCode::CommandTimeout, Some(5));
        retry_suggestions.insert(WebSocketErrorCode::DatabaseError, Some(10));
        retry_suggestions.insert(WebSocketErrorCode::NetworkError, Some(15));
        retry_suggestions.insert(WebSocketErrorCode::ServiceUnavailable, Some(30));
        retry_suggestions.insert(WebSocketErrorCode::RateLimitExceeded, Some(60));
        retry_suggestions.insert(WebSocketErrorCode::TooManyRequests, Some(60));

        Self {
            error_messages,
            retry_suggestions,
            error_stats: Arc::new(RwLock::new(ErrorStats {
                total_errors: 0,
                errors_by_code: HashMap::new(),
                errors_by_severity: HashMap::new(),
                errors_by_category: HashMap::new(),
                recent_errors: Vec::new(),
                last_updated: chrono::Utc::now(),
            })),
            max_recent_errors: 1000,
        }
    }
}

impl WebSocketErrorMapper {
    /// 记录错误到统计信息
    pub fn record_error(&self, error: &WebSocketError) {
        let mut stats = self.error_stats.write().unwrap();
        stats.total_errors += 1;

        // 按错误代码统计
        *stats.errors_by_code.entry(error.code.clone()).or_insert(0) += 1;

        // 按严重程度统计
        *stats.errors_by_severity.entry(error.severity.clone()).or_insert(0) += 1;

        // 按分类统计
        *stats.errors_by_category.entry(error.category.clone()).or_insert(0) += 1;

        // 记录最近的错误
        stats.recent_errors.push(error.clone());
        if stats.recent_errors.len() > self.max_recent_errors {
            stats.recent_errors.remove(0);
        }

        stats.last_updated = chrono::Utc::now();

        // 记录日志
        match error.severity {
            ErrorSeverity::Critical => error!("Critical WebSocket error: {:?}", error),
            ErrorSeverity::High => error!("High severity WebSocket error: {:?}", error),
            ErrorSeverity::Medium => warn!("Medium severity WebSocket error: {:?}", error),
            ErrorSeverity::Low => info!("Low severity WebSocket error: {:?}", error),
        }
    }

    /// 获取错误统计信息
    pub fn get_error_stats(&self) -> ErrorStats {
        self.error_stats.read().unwrap().clone()
    }

    /// 获取严重错误列表
    pub fn get_critical_errors(&self) -> Vec<WebSocketError> {
        let stats = self.error_stats.read().unwrap();
        stats.recent_errors.iter()
            .filter(|e| e.is_critical())
            .cloned()
            .collect()
    }

    /// 获取高优先级错误列表
    pub fn get_high_priority_errors(&self) -> Vec<WebSocketError> {
        let stats = self.error_stats.read().unwrap();
        stats.recent_errors.iter()
            .filter(|e| e.is_high_priority())
            .cloned()
            .collect()
    }

    /// 清理过期的错误记录
    pub fn cleanup_old_errors(&self, max_age: Duration) {
        let mut stats = self.error_stats.write().unwrap();
        let cutoff_time = chrono::Utc::now() - chrono::Duration::from_std(max_age).unwrap();

        stats.recent_errors.retain(|error| error.timestamp > cutoff_time);
        stats.last_updated = chrono::Utc::now();
    }

    /// 从AppError映射到WebSocketError
    pub fn map_app_error(&self, error: &AppError) -> WebSocketError {
        let (code, message) = match error {
            AppError::Validation { message } => {
                (WebSocketErrorCode::ValidationFailed, message.clone())
            }
            AppError::NotFound { resource } => {
                let code = match resource.as_str() {
                    "label" => WebSocketErrorCode::LabelNotFound,
                    "user" => WebSocketErrorCode::UserNotFound,
                    _ => WebSocketErrorCode::CommandNotFound,
                };
                (code, format!("{} not found", resource))
            }
            AppError::Conflict { message, .. } => {
                if message.contains("already exists") {
                    (WebSocketErrorCode::LabelExists, message.clone())
                } else {
                    (WebSocketErrorCode::CommandFailed, message.clone())
                }
            }
            AppError::Auth { message } => {
                (WebSocketErrorCode::AuthenticationFailed, message.clone())
            }
            AppError::Database(_) => {
                (WebSocketErrorCode::DatabaseError, "Database error".to_string())
            }
            AppError::Internal(message) => {
                (WebSocketErrorCode::InternalError, message.clone())
            }
            _ => {
                (WebSocketErrorCode::InternalError, "Unknown error".to_string())
            }
        };

        let mut ws_error = WebSocketError::new(code, message);

        // 添加重试建议
        if let Some(retry_after) = self.retry_suggestions.get(&ws_error.code) {
            if let Some(retry_seconds) = retry_after {
                ws_error.retry_after = Some(*retry_seconds);
            }
        }

        // 记录错误统计
        self.record_error(&ws_error);

        ws_error
    }

    /// 从限流错误映射
    pub fn map_rate_limit_error(&self, retry_after: Option<u64>) -> WebSocketError {
        let mut error = WebSocketError::new(
            WebSocketErrorCode::RateLimitExceeded,
            self.error_messages.get(&WebSocketErrorCode::RateLimitExceeded)
                .unwrap_or(&"Rate limit exceeded".to_string())
                .clone(),
        );

        if let Some(retry_seconds) = retry_after {
            error.retry_after = Some(retry_seconds);
        }

        // 记录错误统计
        self.record_error(&error);

        error
    }

    /// 从超时错误映射
    pub fn map_timeout_error(&self, operation: &str) -> WebSocketError {
        let mut error = WebSocketError::new(
            WebSocketErrorCode::CommandTimeout,
            format!("Operation '{}' timed out", operation),
        );
        error.retry_after = Some(5);

        // 记录错误统计
        self.record_error(&error);

        error
    }

    /// 从重试错误映射
    pub fn map_retry_error(&self, operation: &str, max_retries: u32) -> WebSocketError {
        let mut error = WebSocketError::new(
            WebSocketErrorCode::CommandFailed,
            format!("Operation '{}' failed after {} retries", operation, max_retries),
        );
        error.retry_after = Some(10);

        // 记录错误统计
        self.record_error(&error);

        error
    }

    /// 从认证错误映射
    pub fn map_auth_error(&self, auth_error: &crate::websocket::auth::WebSocketAuthError) -> WebSocketError {
        let (code, message) = match auth_error {
            crate::websocket::auth::WebSocketAuthError::MissingToken => {
                (WebSocketErrorCode::TokenInvalid, "Missing authentication token".to_string())
            }
            crate::websocket::auth::WebSocketAuthError::InvalidToken => {
                (WebSocketErrorCode::TokenInvalid, "Invalid authentication token".to_string())
            }
            crate::websocket::auth::WebSocketAuthError::ExpiredToken => {
                (WebSocketErrorCode::TokenExpired, "Token has expired".to_string())
            }
            crate::websocket::auth::WebSocketAuthError::UserNotFound => {
                (WebSocketErrorCode::UserNotFound, "User not found".to_string())
            }
            crate::websocket::auth::WebSocketAuthError::InvalidUserId => {
                (WebSocketErrorCode::AuthenticationFailed, "Invalid user ID format".to_string())
            }
            crate::websocket::auth::WebSocketAuthError::DatabaseError => {
                (WebSocketErrorCode::DatabaseError, "Database error during authentication".to_string())
            }
        };

        WebSocketError::new(code, message)
    }

    /// 获取用户友好的错误消息
    pub fn get_user_friendly_message(&self, code: &WebSocketErrorCode) -> String {
        self.error_messages.get(code)
            .cloned()
            .unwrap_or_else(|| "Unknown error occurred".to_string())
    }

    /// 检查错误是否应该重试
    pub fn should_retry(&self, error: &WebSocketError) -> bool {
        matches!(
            error.code,
            WebSocketErrorCode::CommandTimeout
                | WebSocketErrorCode::DatabaseError
                | WebSocketErrorCode::NetworkError
                | WebSocketErrorCode::ServiceUnavailable
                | WebSocketErrorCode::InternalError
        )
    }

    /// 检查错误是否应该断开连接
    pub fn should_disconnect(&self, error: &WebSocketError) -> bool {
        matches!(
            error.code,
            WebSocketErrorCode::AuthenticationFailed
                | WebSocketErrorCode::TokenExpired
                | WebSocketErrorCode::TokenInvalid
                | WebSocketErrorCode::UserNotFound
                | WebSocketErrorCode::ConnectionLost
                | WebSocketErrorCode::ConnectionTimeout
        )
    }
}

/// 错误处理器
#[derive(Clone)]
pub struct WebSocketErrorHandler {
    mapper: WebSocketErrorMapper,
}

impl Default for WebSocketErrorHandler {
    fn default() -> Self {
        Self {
            mapper: WebSocketErrorMapper::default(),
        }
    }
}

impl WebSocketErrorHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// 处理AppError
    pub fn handle_app_error(&self, error: &AppError) -> WebSocketError {
        self.mapper.map_app_error(error)
    }

    /// 处理限流错误
    pub fn handle_rate_limit_error(&self, retry_after: Option<u64>) -> WebSocketError {
        self.mapper.map_rate_limit_error(retry_after)
    }

    /// 处理超时错误
    pub fn handle_timeout_error(&self, operation: &str) -> WebSocketError {
        self.mapper.map_timeout_error(operation)
    }

    /// 处理重试错误
    pub fn handle_retry_error(&self, operation: &str, max_retries: u32) -> WebSocketError {
        self.mapper.map_retry_error(operation, max_retries)
    }

    /// 处理认证错误
    pub fn handle_auth_error(&self, auth_error: &crate::websocket::auth::WebSocketAuthError) -> WebSocketError {
        self.mapper.map_auth_error(auth_error)
    }

    /// 检查是否应该重试
    pub fn should_retry(&self, error: &WebSocketError) -> bool {
        self.mapper.should_retry(error)
    }

    /// 检查是否应该断开连接
    pub fn should_disconnect(&self, error: &WebSocketError) -> bool {
        self.mapper.should_disconnect(error)
    }

    /// 获取错误的重试延迟
    pub fn get_retry_delay(&self, error: &WebSocketError) -> Option<u64> {
        error.retry_after
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_mapper_app_error() {
        let mapper = WebSocketErrorMapper::default();

        let app_error = AppError::not_found("label");
        let ws_error = mapper.map_app_error(&app_error);

        assert_eq!(ws_error.code, WebSocketErrorCode::LabelNotFound);
        assert!(ws_error.message.contains("label"));
    }

    #[test]
    fn test_error_mapper_validation_error() {
        let mapper = WebSocketErrorMapper::default();

        let app_error = AppError::validation("Validation failed");
        let ws_error = mapper.map_app_error(&app_error);

        assert_eq!(ws_error.code, WebSocketErrorCode::ValidationFailed);
        // details 不再强制要求
    }

    #[test]
    fn test_error_mapper_rate_limit() {
        let mapper = WebSocketErrorMapper::default();

        let ws_error = mapper.map_rate_limit_error(Some(60));

        assert_eq!(ws_error.code, WebSocketErrorCode::RateLimitExceeded);
        assert_eq!(ws_error.retry_after, Some(60));
    }

    #[test]
    fn test_error_mapper_timeout() {
        let mapper = WebSocketErrorMapper::default();

        let ws_error = mapper.map_timeout_error("create_label");

        assert_eq!(ws_error.code, WebSocketErrorCode::CommandTimeout);
        assert!(ws_error.message.contains("create_label"));
        assert_eq!(ws_error.retry_after, Some(5));
    }

    #[test]
    fn test_should_retry_logic() {
        let mapper = WebSocketErrorMapper::default();

        let retryable_error = WebSocketError::new(WebSocketErrorCode::DatabaseError, "DB error".to_string());
        assert!(mapper.should_retry(&retryable_error));

        let non_retryable_error = WebSocketError::new(WebSocketErrorCode::ValidationFailed, "Validation error".to_string());
        assert!(!mapper.should_retry(&non_retryable_error));
    }

    #[test]
    fn test_should_disconnect_logic() {
        let mapper = WebSocketErrorMapper::default();

        let disconnect_error = WebSocketError::new(WebSocketErrorCode::TokenExpired, "Token expired".to_string());
        assert!(mapper.should_disconnect(&disconnect_error));

        let non_disconnect_error = WebSocketError::new(WebSocketErrorCode::ValidationFailed, "Validation error".to_string());
        assert!(!mapper.should_disconnect(&non_disconnect_error));
    }

    #[test]
    fn test_error_handler() {
        let handler = WebSocketErrorHandler::new();

        let app_error = AppError::internal("Internal error".to_string());
        let ws_error = handler.handle_app_error(&app_error);

        assert_eq!(ws_error.code, WebSocketErrorCode::InternalError);
        assert!(handler.should_retry(&ws_error));
    }
}
