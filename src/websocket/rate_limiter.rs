use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// 限流配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 时间窗口（秒）
    pub window_seconds: u64,
    /// 最大请求数
    pub max_requests: u32,
    /// 命令类型特定的限制
    pub command_limits: HashMap<String, u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut command_limits = HashMap::new();
        command_limits.insert("create_label".to_string(), 10);
        command_limits.insert("update_label".to_string(), 20);
        command_limits.insert("delete_label".to_string(), 5);
        command_limits.insert("ping".to_string(), 60);

        Self {
            window_seconds: 60, // 1分钟窗口
            max_requests: 100,  // 默认最大100个请求
            command_limits,
        }
    }
}

/// 用户请求记录
#[derive(Debug, Clone)]
struct UserRequestRecord {
    requests: Vec<Instant>,
    command_counts: HashMap<String, Vec<Instant>>,
}

impl UserRequestRecord {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            command_counts: HashMap::new(),
        }
    }

    /// 清理过期请求
    fn cleanup_expired(&mut self, window_duration: Duration) {
        let cutoff = Instant::now() - window_duration;
        self.requests.retain(|&time| time > cutoff);

        for (_, times) in self.command_counts.iter_mut() {
            times.retain(|&time| time > cutoff);
        }
    }

    /// 添加请求记录
    fn add_request(&mut self, command_type: Option<&str>) {
        let now = Instant::now();
        self.requests.push(now);

        if let Some(cmd_type) = command_type {
            self.command_counts
                .entry(cmd_type.to_string())
                .or_insert_with(Vec::new)
                .push(now);
        }
    }

    /// 检查是否超过限制
    fn is_rate_limited(&self, config: &RateLimitConfig, command_type: Option<&str>) -> bool {
        let window_duration = Duration::from_secs(config.window_seconds);
        let cutoff = Instant::now() - window_duration;

        // 检查总请求数限制
        let recent_requests = self.requests.iter().filter(|&&time| time > cutoff).count();
        if recent_requests >= config.max_requests as usize {
            return true;
        }

        // 检查命令特定限制
        if let Some(cmd_type) = command_type {
            if let Some(command_limit) = config.command_limits.get(cmd_type) {
                if let Some(command_times) = self.command_counts.get(cmd_type) {
                    let recent_command_requests = command_times.iter().filter(|&&time| time > cutoff).count();
                    if recent_command_requests >= *command_limit as usize {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// WebSocket限流器
pub struct WebSocketRateLimiter {
    /// 用户请求记录
    user_records: Arc<RwLock<HashMap<Uuid, UserRequestRecord>>>,
    /// 限流配置
    config: RateLimitConfig,
}

impl WebSocketRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            user_records: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// 检查用户是否被限流
    pub async fn is_rate_limited(&self, user_id: Uuid, command_type: Option<&str>) -> bool {
        let mut records = self.user_records.write().await;

        // 获取或创建用户记录
        let record = records.entry(user_id).or_insert_with(UserRequestRecord::new);

        // 清理过期记录
        record.cleanup_expired(Duration::from_secs(self.config.window_seconds));

        // 检查是否被限流
        let is_limited = record.is_rate_limited(&self.config, command_type);

        // 如果没被限流，添加请求记录
        if !is_limited {
            record.add_request(command_type);
        }

        is_limited
    }

    /// 获取用户当前请求统计
    pub async fn get_user_stats(&self, user_id: Uuid) -> Option<UserStats> {
        let records = self.user_records.read().await;
        let record = records.get(&user_id)?;

        let window_duration = Duration::from_secs(self.config.window_seconds);
        let cutoff = Instant::now() - window_duration;

        let total_requests = record.requests.iter().filter(|&&time| time > cutoff).count();
        let mut command_stats = HashMap::new();

        for (cmd_type, times) in &record.command_counts {
            let count = times.iter().filter(|&&time| time > cutoff).count();
            command_stats.insert(cmd_type.clone(), count);
        }

        Some(UserStats {
            user_id,
            total_requests,
            command_stats,
            window_seconds: self.config.window_seconds,
        })
    }

    /// 清理过期记录
    pub async fn cleanup_expired(&self) {
        let mut records = self.user_records.write().await;
        let window_duration = Duration::from_secs(self.config.window_seconds);

        records.retain(|_, record| {
            record.cleanup_expired(window_duration);
            !record.requests.is_empty() || !record.command_counts.is_empty()
        });
    }

    /// 重置用户限制
    pub async fn reset_user_limit(&self, user_id: Uuid) {
        let mut records = self.user_records.write().await;
        records.remove(&user_id);
    }

    /// 启动清理任务
    pub async fn start_cleanup_task(&self) {
        let limiter = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // 每分钟清理一次
            loop {
                interval.tick().await;
                limiter.cleanup_expired().await;
            }
        });
    }
}

impl Clone for WebSocketRateLimiter {
    fn clone(&self) -> Self {
        Self {
            user_records: self.user_records.clone(),
            config: self.config.clone(),
        }
    }
}

/// 用户统计信息
#[derive(Debug, Clone)]
pub struct UserStats {
    pub user_id: Uuid,
    pub total_requests: usize,
    pub command_stats: HashMap<String, usize>,
    pub window_seconds: u64,
}

/// 限流错误
#[derive(Debug, Clone)]
pub struct RateLimitError {
    pub message: String,
    pub retry_after: Option<u64>,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RateLimitError {}

#[cfg(test)]
mod tests {
    use super::*;
    // use std::thread;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let config = RateLimitConfig {
            window_seconds: 60,
            max_requests: 5,
            command_limits: HashMap::new(),
        };

        let limiter = WebSocketRateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // 前5个请求应该通过
        for _ in 0..5 {
            assert!(!limiter.is_rate_limited(user_id, None).await);
        }

        // 第6个请求应该被限流
        assert!(limiter.is_rate_limited(user_id, None).await);
    }

    #[tokio::test]
    async fn test_command_specific_limits() {
        let mut command_limits = HashMap::new();
        command_limits.insert("create_label".to_string(), 2);

        let config = RateLimitConfig {
            window_seconds: 60,
            max_requests: 100,
            command_limits,
        };

        let limiter = WebSocketRateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // 前2个create_label请求应该通过
        assert!(!limiter.is_rate_limited(user_id, Some("create_label")).await);
        assert!(!limiter.is_rate_limited(user_id, Some("create_label")).await);

        // 第3个create_label请求应该被限流
        assert!(limiter.is_rate_limited(user_id, Some("create_label")).await);

        // 但其他命令应该不受影响
        assert!(!limiter.is_rate_limited(user_id, Some("update_label")).await);
    }

    #[tokio::test]
    async fn test_user_stats() {
        let config = RateLimitConfig::default();
        let limiter = WebSocketRateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // 添加一些请求
        limiter.is_rate_limited(user_id, Some("create_label")).await;
        limiter.is_rate_limited(user_id, Some("create_label")).await;
        limiter.is_rate_limited(user_id, Some("update_label")).await;

        let stats = limiter.get_user_stats(user_id).await.unwrap();
        assert_eq!(stats.user_id, user_id);
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.command_stats.get("create_label"), Some(&2));
        assert_eq!(stats.command_stats.get("update_label"), Some(&1));
    }

    #[tokio::test]
    async fn test_reset_user_limit() {
        let config = RateLimitConfig {
            window_seconds: 60,
            max_requests: 2,
            command_limits: HashMap::new(),
        };

        let limiter = WebSocketRateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // 达到限制
        limiter.is_rate_limited(user_id, None).await;
        limiter.is_rate_limited(user_id, None).await;
        assert!(limiter.is_rate_limited(user_id, None).await);

        // 重置限制
        limiter.reset_user_limit(user_id).await;

        // 应该可以继续请求
        assert!(!limiter.is_rate_limited(user_id, None).await);
    }
}
