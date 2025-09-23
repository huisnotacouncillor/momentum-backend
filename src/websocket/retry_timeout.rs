use std::time::Duration;
use tokio::time::timeout;
use crate::error::AppError;

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始延迟（毫秒）
    pub initial_delay_ms: u64,
    /// 延迟倍数（指数退避）
    pub delay_multiplier: f64,
    /// 最大延迟（毫秒）
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            delay_multiplier: 2.0,
            max_delay_ms: 5000,
        }
    }
}

/// 超时配置
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// 命令执行超时时间（秒）
    pub command_timeout_seconds: u64,
    /// 连接超时时间（秒）
    pub connection_timeout_seconds: u64,
    /// 心跳超时时间（秒）
    pub heartbeat_timeout_seconds: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            command_timeout_seconds: 30,
            connection_timeout_seconds: 60,
            heartbeat_timeout_seconds: 30,
        }
    }
}

/// 重试和超时管理器
#[derive(Clone)]
pub struct RetryTimeoutManager {
    retry_config: RetryConfig,
    timeout_config: TimeoutConfig,
}

impl RetryTimeoutManager {
    pub fn new(retry_config: RetryConfig, timeout_config: TimeoutConfig) -> Self {
        Self {
            retry_config,
            timeout_config,
        }
    }

    /// 执行带重试和超时的操作
    pub async fn execute_with_retry<F, T, E>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> Result<T, RetryTimeoutError>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
        E: std::fmt::Display + Send + Sync + 'static,
    {
        let mut _last_error = None;
        let mut delay_ms = self.retry_config.initial_delay_ms;

        for attempt in 0..=self.retry_config.max_retries {
            // 执行操作并设置超时
            let operation_future = operation();
            let timeout_duration = Duration::from_secs(self.timeout_config.command_timeout_seconds);

            match timeout(timeout_duration, operation_future).await {
                Ok(Ok(result)) => {
                    if attempt > 0 {
                        tracing::info!("Operation '{}' succeeded after {} retries", operation_name, attempt);
                    }
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    _last_error = Some(RetryTimeoutError::OperationError(e.to_string()));
                    tracing::warn!("Operation '{}' failed on attempt {}: {}", operation_name, attempt + 1, e);
                }
                Err(_e) => {
                    _last_error = Some(RetryTimeoutError::Timeout(format!(
                        "Operation '{}' timed out after {} seconds",
                        operation_name, self.timeout_config.command_timeout_seconds
                    )));
                    tracing::warn!("Operation '{}' timed out on attempt {}", operation_name, attempt + 1);
                }
            }

            // 如果不是最后一次尝试，等待后重试
            if attempt < self.retry_config.max_retries {
                let delay_duration = Duration::from_millis(delay_ms);
                tracing::debug!("Waiting {}ms before retry {} for operation '{}'", delay_ms, attempt + 2, operation_name);
                tokio::time::sleep(delay_duration).await;

                // 计算下次延迟时间（指数退避）
                delay_ms = ((delay_ms as f64) * self.retry_config.delay_multiplier) as u64;
                delay_ms = delay_ms.min(self.retry_config.max_delay_ms);
            }
        }

        Err(RetryTimeoutError::MaxRetriesExceeded(format!(
            "Operation '{}' failed after {} retries",
            operation_name, self.retry_config.max_retries
        )))
    }

    /// 执行带超时的操作（不重试）
    pub async fn execute_with_timeout<F, T>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> Result<T, RetryTimeoutError>
    where
        F: std::future::Future<Output = T>,
    {
        let timeout_duration = Duration::from_secs(self.timeout_config.command_timeout_seconds);

        match timeout(timeout_duration, operation).await {
            Ok(result) => Ok(result),
            Err(_) => Err(RetryTimeoutError::Timeout(format!(
                "Operation '{}' timed out after {} seconds",
                operation_name, self.timeout_config.command_timeout_seconds
            ))),
        }
    }

    /// 检查是否应该重试的错误
    pub fn should_retry(&self, error: &AppError) -> bool {
        match error {
            AppError::Internal(_) => true,
            AppError::Database(_) => true,
            _ => false, // 业务逻辑错误通常不需要重试
        }
    }

    /// 获取重试配置
    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    /// 获取超时配置
    pub fn timeout_config(&self) -> &TimeoutConfig {
        &self.timeout_config
    }
}

/// 重试和超时错误
#[derive(Debug, Clone)]
pub enum RetryTimeoutError {
    /// 操作错误
    OperationError(String),
    /// 超时错误
    Timeout(String),
    /// 超过最大重试次数
    MaxRetriesExceeded(String),
}

impl std::fmt::Display for RetryTimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetryTimeoutError::OperationError(msg) => write!(f, "Operation error: {}", msg),
            RetryTimeoutError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            RetryTimeoutError::MaxRetriesExceeded(msg) => write!(f, "Max retries exceeded: {}", msg),
        }
    }
}

impl std::error::Error for RetryTimeoutError {}

/// 连接健康检查器
pub struct ConnectionHealthChecker {
    timeout_config: TimeoutConfig,
}

impl ConnectionHealthChecker {
    pub fn new(timeout_config: TimeoutConfig) -> Self {
        Self { timeout_config }
    }

    /// 检查连接是否健康
    pub async fn check_connection_health<F>(
        &self,
        health_check: F,
    ) -> Result<bool, RetryTimeoutError>
    where
        F: std::future::Future<Output = Result<bool, String>>,
    {
        let timeout_duration = Duration::from_secs(self.timeout_config.connection_timeout_seconds);

        match timeout(timeout_duration, health_check).await {
            Ok(Ok(is_healthy)) => Ok(is_healthy),
            Ok(Err(e)) => Err(RetryTimeoutError::OperationError(e)),
            Err(_) => Err(RetryTimeoutError::Timeout(format!(
                "Connection health check timed out after {} seconds",
                self.timeout_config.connection_timeout_seconds
            ))),
        }
    }

    /// 检查心跳是否正常
    pub async fn check_heartbeat<F>(
        &self,
        heartbeat_check: F,
    ) -> Result<bool, RetryTimeoutError>
    where
        F: std::future::Future<Output = Result<bool, String>>,
    {
        let timeout_duration = Duration::from_secs(self.timeout_config.heartbeat_timeout_seconds);

        match timeout(timeout_duration, heartbeat_check).await {
            Ok(Ok(is_alive)) => Ok(is_alive),
            Ok(Err(e)) => Err(RetryTimeoutError::OperationError(e)),
            Err(_) => Err(RetryTimeoutError::Timeout(format!(
                "Heartbeat check timed out after {} seconds",
                self.timeout_config.heartbeat_timeout_seconds
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 10,
            delay_multiplier: 2.0,
            max_delay_ms: 1000,
        };
        let timeout_config = TimeoutConfig::default();
        let manager = RetryTimeoutManager::new(retry_config, timeout_config);

        let attempt_count = Arc::new(AtomicU32::new(0));

        let result = manager.execute_with_retry(
            || {
                let count = attempt_count.clone();
                Box::pin(async move {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current == 0 {
                        Err("First attempt fails")
                    } else {
                        Ok("Success")
                    }
                })
            },
            "test_operation",
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
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
            RetryTimeoutError::MaxRetriesExceeded(_) => {},
            _ => panic!("Expected MaxRetriesExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_timeout() {
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
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    Ok::<String, String>("Should not reach here".to_string())
                })
            },
            "test_operation",
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RetryTimeoutError::Timeout(_) | RetryTimeoutError::MaxRetriesExceeded(_) => {},
            _ => panic!("Expected Timeout or MaxRetriesExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_should_retry_logic() {
        let manager = RetryTimeoutManager::new(RetryConfig::default(), TimeoutConfig::default());

        assert!(manager.should_retry(&AppError::Database(diesel::result::Error::NotFound)));
        assert!(manager.should_retry(&AppError::internal("Internal error")));
        assert!(!manager.should_retry(&AppError::validation("v")));
        assert!(!manager.should_retry(&AppError::not_found("Not found")));
    }

    #[tokio::test]
    async fn test_connection_health_checker() {
        let timeout_config = TimeoutConfig {
            connection_timeout_seconds: 1,
            command_timeout_seconds: 30,
            heartbeat_timeout_seconds: 30,
        };
        let checker = ConnectionHealthChecker::new(timeout_config);

        // 成功的健康检查
        let result = checker.check_connection_health(async {
            Ok(true)
        }).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // 超时的健康检查
        let result = checker.check_connection_health(async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok(true)
        }).await;
        assert!(result.is_err());
    }
}
