//! 事件中间件系统
//!
//! 提供事件处理的中间件机制，支持认证、授权、日志、监控、缓存等横切关注点

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{Event, EventContext, EventError, EventResult, EventType};
use crate::websocket::auth::AuthenticatedUser;

/// 事件中间件特征
#[async_trait]
pub trait EventMiddleware: Send + Sync {
    /// 中间件名称
    fn name(&self) -> &'static str;

    /// 中间件优先级（数字越小优先级越高）
    fn priority(&self) -> u32 {
        100
    }

    /// 是否启用
    fn enabled(&self) -> bool {
        true
    }

    /// 事件处理前执行
    async fn before_handle(
        &self,
        event: &dyn Event,
        context: &mut EventContext,
    ) -> Result<(), EventError>;

    /// 事件处理后执行
    async fn after_handle(
        &self,
        event: &dyn Event,
        context: &EventContext,
        result: &EventResult,
    ) -> Result<(), EventError>;

    /// 错误处理
    async fn on_error(
        &self,
        event: &dyn Event,
        context: &EventContext,
        error: &EventError,
    ) -> Result<(), EventError> {
        // 默认不做处理
        Ok(())
    }

    /// 中间件描述
    fn description(&self) -> &'static str {
        "Event middleware"
    }

    /// 是否应该处理此事件
    fn should_handle(&self, event: &dyn Event) -> bool {
        true
    }
}

/// 中间件链
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn EventMiddleware>>,
    stats: Arc<RwLock<MiddlewareStats>>,
}

#[derive(Debug, Default)]
pub struct MiddlewareStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub middleware_stats: HashMap<String, MiddlewareExecutionStats>,
}

#[derive(Debug, Default, Clone)]
pub struct MiddlewareExecutionStats {
    pub executions: u64,
    pub successes: u64,
    pub failures: u64,
    pub average_execution_time_ms: f64,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
            stats: Arc::new(RwLock::new(MiddlewareStats::default())),
        }
    }

    /// 添加中间件
    pub fn add(&mut self, middleware: Arc<dyn EventMiddleware>) {
        info!("Adding middleware: {}", middleware.name());
        self.middlewares.push(middleware);
        // 按优先级排序
        self.middlewares.sort_by_key(|m| m.priority());
    }

    /// 批量添加中间件
    pub fn add_batch(&mut self, middlewares: Vec<Arc<dyn EventMiddleware>>) {
        for middleware in middlewares {
            self.add(middleware);
        }
    }

    /// 执行前置中间件
    pub async fn execute_before(
        &self,
        event: &dyn Event,
        context: &mut EventContext,
    ) -> Result<(), EventError> {
        for middleware in &self.middlewares {
            if !middleware.enabled() || !middleware.should_handle(event) {
                continue;
            }

            let start_time = Instant::now();
            let middleware_name = middleware.name().to_string();

            match middleware.before_handle(event, context).await {
                Ok(()) => {
                    self.update_stats(&middleware_name, true, start_time.elapsed())
                        .await;
                    debug!("Middleware {} before_handle succeeded", middleware_name);
                }
                Err(e) => {
                    self.update_stats(&middleware_name, false, start_time.elapsed())
                        .await;
                    error!("Middleware {} before_handle failed: {}", middleware_name, e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// 执行后置中间件
    pub async fn execute_after(
        &self,
        event: &dyn Event,
        context: &EventContext,
        result: &EventResult,
    ) -> Result<(), EventError> {
        // 反向执行后置中间件
        for middleware in self.middlewares.iter().rev() {
            if !middleware.enabled() || !middleware.should_handle(event) {
                continue;
            }

            let start_time = Instant::now();
            let middleware_name = middleware.name().to_string();

            match middleware.after_handle(event, context, result).await {
                Ok(()) => {
                    self.update_stats(&middleware_name, true, start_time.elapsed())
                        .await;
                    debug!("Middleware {} after_handle succeeded", middleware_name);
                }
                Err(e) => {
                    self.update_stats(&middleware_name, false, start_time.elapsed())
                        .await;
                    warn!("Middleware {} after_handle failed: {}", middleware_name, e);
                    // 后置中间件错误不应该阻止流程，只记录日志
                }
            }
        }
        Ok(())
    }

    /// 执行错误处理中间件
    pub async fn execute_on_error(
        &self,
        event: &dyn Event,
        context: &EventContext,
        error: &EventError,
    ) -> Result<(), EventError> {
        for middleware in &self.middlewares {
            if !middleware.enabled() || !middleware.should_handle(event) {
                continue;
            }

            let middleware_name = middleware.name().to_string();
            if let Err(e) = middleware.on_error(event, context, error).await {
                warn!("Middleware {} error handler failed: {}", middleware_name, e);
            }
        }
        Ok(())
    }

    /// 更新统计信息
    async fn update_stats(&self, middleware_name: &str, success: bool, duration: Duration) {
        let mut stats = self.stats.write().await;

        stats.total_executions += 1;
        if success {
            stats.successful_executions += 1;
        } else {
            stats.failed_executions += 1;
        }

        let middleware_stats = stats
            .middleware_stats
            .entry(middleware_name.to_string())
            .or_insert_with(MiddlewareExecutionStats::default);

        middleware_stats.executions += 1;
        if success {
            middleware_stats.successes += 1;
        } else {
            middleware_stats.failures += 1;
        }

        // 更新平均执行时间
        let current_avg = middleware_stats.average_execution_time_ms;
        let current_duration_ms = duration.as_millis() as f64;
        middleware_stats.average_execution_time_ms =
            (current_avg * (middleware_stats.executions - 1) as f64 + current_duration_ms)
                / middleware_stats.executions as f64;

        middleware_stats.last_execution = Some(chrono::Utc::now());
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> MiddlewareStats {
        self.stats.read().await.clone()
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// 认证中间件
pub struct AuthenticationMiddleware;

#[async_trait]
impl EventMiddleware for AuthenticationMiddleware {
    fn name(&self) -> &'static str {
        "authentication"
    }

    fn priority(&self) -> u32 {
        10 // 高优先级
    }

    async fn before_handle(
        &self,
        event: &dyn Event,
        context: &mut EventContext,
    ) -> Result<(), EventError> {
        // 检查是否已有用户信息
        if context.user.is_some() {
            debug!("User already authenticated in context");
            return Ok(());
        }

        // 从事件中提取用户信息（如果有的话）
        if let Some(user_id) = event.user_id() {
            // 这里应该从数据库或缓存中获取用户信息
            // 暂时创建一个模拟用户
            let authenticated_user = AuthenticatedUser {
                user_id,
                username: format!("user_{}", user_id),
                email: format!("user_{}@example.com", user_id),
                name: format!("User {}", user_id),
                avatar_url: None,
                current_workspace_id: None,
            };

            context.user = Some(authenticated_user);
            info!("User {} authenticated", user_id);
        } else {
            warn!("No user information found in event");
            return Err(EventError::PermissionError(
                "User authentication required".to_string(),
            ));
        }

        Ok(())
    }

    async fn after_handle(
        &self,
        _event: &dyn Event,
        _context: &EventContext,
        _result: &EventResult,
    ) -> Result<(), EventError> {
        // 认证中间件在后置处理中通常不需要做什么
        Ok(())
    }

    fn description(&self) -> &'static str {
        "Authenticates users for event processing"
    }
}

/// 权限检查中间件
pub struct AuthorizationMiddleware {
    permission_cache: Arc<RwLock<HashMap<String, bool>>>,
}

impl AuthorizationMiddleware {
    pub fn new() -> Self {
        Self {
            permission_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn check_permission(&self, user_id: Uuid, permission: &str) -> bool {
        // 检查缓存
        let cache_key = format!("{}:{}", user_id, permission);
        {
            let cache = self.permission_cache.read().await;
            if let Some(&cached_result) = cache.get(&cache_key) {
                return cached_result;
            }
        }

        // 实际的权限检查逻辑
        let has_permission = self.check_permission_from_db(user_id, permission).await;

        // 更新缓存
        {
            let mut cache = self.permission_cache.write().await;
            cache.insert(cache_key, has_permission);
        }

        has_permission
    }

    async fn check_permission_from_db(&self, _user_id: Uuid, _permission: &str) -> bool {
        // TODO: 实现真正的权限检查逻辑
        // 这里应该查询数据库中的用户角色和权限
        true // 暂时返回true
    }
}

#[async_trait]
impl EventMiddleware for AuthorizationMiddleware {
    fn name(&self) -> &'static str {
        "authorization"
    }

    fn priority(&self) -> u32 {
        20 // 在认证之后
    }

    async fn before_handle(
        &self,
        event: &dyn Event,
        context: &mut EventContext,
    ) -> Result<(), EventError> {
        let user = context
            .user
            .as_ref()
            .ok_or_else(|| EventError::PermissionError("User not authenticated".to_string()))?;

        // 获取所需权限
        let required_permissions = event.required_permissions();
        if required_permissions.is_empty() {
            return Ok(());
        }

        // 检查权限
        for permission in required_permissions {
            if !self.check_permission(user.user_id, &permission).await {
                return Err(EventError::PermissionError(format!(
                    "User {} lacks permission: {}",
                    user.user_id, permission
                )));
            }
        }

        info!("Authorization check passed for user {}", user.user_id);
        Ok(())
    }

    async fn after_handle(
        &self,
        _event: &dyn Event,
        _context: &EventContext,
        _result: &EventResult,
    ) -> Result<(), EventError> {
        Ok(())
    }

    fn description(&self) -> &'static str {
        "Checks user permissions for event processing"
    }
}

/// 日志中间件
pub struct LoggingMiddleware {
    log_level: LogLevel,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LoggingMiddleware {
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }

    fn should_log(&self, level: &LogLevel) -> bool {
        use LogLevel::*;
        match (level, &self.log_level) {
            (Debug, Debug) => true,
            (Info, Debug | Info) => true,
            (Warn, Debug | Info | Warn) => true,
            (Error, _) => true,
            _ => false,
        }
    }
}

#[async_trait]
impl EventMiddleware for LoggingMiddleware {
    fn name(&self) -> &'static str {
        "logging"
    }

    fn priority(&self) -> u32 {
        30
    }

    async fn before_handle(
        &self,
        event: &dyn Event,
        context: &mut EventContext,
    ) -> Result<(), EventError> {
        if self.should_log(&LogLevel::Info) {
            info!(
                "Processing event: {} (type: {:?}, user: {:?}, request: {})",
                event.event_id(),
                event.event_type(),
                context.user_id(),
                context.request_id
            );
        }

        // 记录开始时间
        context.attributes.insert(
            "log_start_time".to_string(),
            serde_json::json!(context.created_at.elapsed().as_millis()),
        );

        Ok(())
    }

    async fn after_handle(
        &self,
        event: &dyn Event,
        context: &EventContext,
        result: &EventResult,
    ) -> Result<(), EventError> {
        let execution_time = context.execution_time();

        if result.success {
            if self.should_log(&LogLevel::Info) {
                info!(
                    "Event {} completed successfully in {:?} (handler: {})",
                    event.event_id(),
                    execution_time,
                    result.handler_name
                );
            }
        } else {
            if self.should_log(&LogLevel::Warn) {
                warn!(
                    "Event {} failed in {:?} (handler: {}, error: {:?})",
                    event.event_id(),
                    execution_time,
                    result.handler_name,
                    result.error
                );
            }
        }

        Ok(())
    }

    async fn on_error(
        &self,
        event: &dyn Event,
        context: &EventContext,
        error: &EventError,
    ) -> Result<(), EventError> {
        error!(
            "Event {} processing error: {} (request: {}, user: {:?})",
            event.event_id(),
            error,
            context.request_id,
            context.user_id()
        );
        Ok(())
    }

    fn description(&self) -> &'static str {
        "Logs event processing information"
    }
}

/// 性能监控中间件
pub struct PerformanceMiddleware {
    slow_threshold_ms: u64,
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    pub total_events: u64,
    pub slow_events: u64,
    pub average_execution_time_ms: f64,
    pub max_execution_time_ms: u64,
    pub min_execution_time_ms: u64,
    pub events_by_type: HashMap<EventType, u64>,
}

impl PerformanceMiddleware {
    pub fn new(slow_threshold_ms: u64) -> Self {
        Self {
            slow_threshold_ms,
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                min_execution_time_ms: u64::MAX,
                ..Default::default()
            })),
        }
    }

    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = PerformanceMetrics {
            min_execution_time_ms: u64::MAX,
            ..Default::default()
        };
    }
}

#[async_trait]
impl EventMiddleware for PerformanceMiddleware {
    fn name(&self) -> &'static str {
        "performance"
    }

    fn priority(&self) -> u32 {
        40
    }

    async fn before_handle(
        &self,
        _event: &dyn Event,
        _context: &mut EventContext,
    ) -> Result<(), EventError> {
        // 开始时间已在context中记录
        Ok(())
    }

    async fn after_handle(
        &self,
        event: &dyn Event,
        _context: &EventContext,
        result: &EventResult,
    ) -> Result<(), EventError> {
        let execution_time_ms = result.execution_time_ms;
        let event_type = event.event_type();

        // 更新性能指标
        let mut metrics = self.metrics.write().await;
        metrics.total_events += 1;

        if execution_time_ms > self.slow_threshold_ms {
            metrics.slow_events += 1;
            warn!(
                "Slow event detected: {} took {}ms (threshold: {}ms)",
                event.event_id(),
                execution_time_ms,
                self.slow_threshold_ms
            );
        }

        // 更新平均执行时间
        let current_avg = metrics.average_execution_time_ms;
        metrics.average_execution_time_ms = (current_avg * (metrics.total_events - 1) as f64
            + execution_time_ms as f64)
            / metrics.total_events as f64;

        // 更新最大最小值
        metrics.max_execution_time_ms = metrics.max_execution_time_ms.max(execution_time_ms);
        if metrics.min_execution_time_ms == u64::MAX {
            metrics.min_execution_time_ms = execution_time_ms;
        } else {
            metrics.min_execution_time_ms = metrics.min_execution_time_ms.min(execution_time_ms);
        }

        // 按类型统计
        *metrics.events_by_type.entry(event_type).or_insert(0) += 1;

        Ok(())
    }

    fn description(&self) -> &'static str {
        "Monitors event processing performance"
    }
}

/// 缓存中间件
pub struct CacheMiddleware {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    ttl_seconds: u64,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: EventResult,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl CacheMiddleware {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl_seconds,
        }
    }

    fn generate_cache_key(&self, event: &dyn Event) -> Option<String> {
        // 只缓存可缓存的事件
        if event.is_cacheable() {
            Some(format!(
                "{}:{}:{}",
                event.event_type() as u8,
                event.event_id(),
                // 这里可以添加更多参数来生成缓存键
                event.cache_key_params().join(":")
            ))
        } else {
            None
        }
    }

    async fn get_cached_result(&self, cache_key: &str) -> Option<EventResult> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(cache_key) {
            if chrono::Utc::now() < entry.expires_at {
                return Some(entry.value.clone());
            }
        }
        None
    }

    async fn cache_result(&self, cache_key: String, result: EventResult) {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(self.ttl_seconds as i64);
        let entry = CacheEntry {
            value: result,
            expires_at,
        };

        let mut cache = self.cache.write().await;
        cache.insert(cache_key, entry);
    }

    async fn cleanup_expired(&self) {
        let now = chrono::Utc::now();
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| now < entry.expires_at);
    }
}

#[async_trait]
impl EventMiddleware for CacheMiddleware {
    fn name(&self) -> &'static str {
        "cache"
    }

    fn priority(&self) -> u32 {
        50
    }

    async fn before_handle(
        &self,
        event: &dyn Event,
        context: &mut EventContext,
    ) -> Result<(), EventError> {
        if let Some(cache_key) = self.generate_cache_key(event) {
            if let Some(cached_result) = self.get_cached_result(&cache_key).await {
                // 将缓存结果存储在上下文中，供后续使用
                context
                    .set_execution_data(
                        "cached_result".to_string(),
                        serde_json::to_value(&cached_result).unwrap(),
                    )
                    .await;

                debug!("Cache hit for event: {}", event.event_id());
            }
        }

        // 定期清理过期缓存
        tokio::spawn({
            let cache = self.cache.clone();
            async move {
                let mut cache = cache.write().await;
                let now = chrono::Utc::now();
                cache.retain(|_, entry| now < entry.expires_at);
            }
        });

        Ok(())
    }

    async fn after_handle(
        &self,
        event: &dyn Event,
        context: &EventContext,
        result: &EventResult,
    ) -> Result<(), EventError> {
        // 检查是否有缓存的结果
        if context.get_execution_data("cached_result").await.is_some() {
            return Ok(()); // 使用了缓存，不需要再次缓存
        }

        // 缓存新结果
        if let Some(cache_key) = self.generate_cache_key(event) {
            if result.success {
                self.cache_result(cache_key, result.clone()).await;
                debug!("Cached result for event: {}", event.event_id());
            }
        }

        Ok(())
    }

    fn description(&self) -> &'static str {
        "Caches event processing results"
    }

    fn should_handle(&self, event: &dyn Event) -> bool {
        event.is_cacheable()
    }
}

/// 限流中间件
pub struct RateLimitingMiddleware {
    limits: Arc<RwLock<HashMap<String, RateLimit>>>,
    default_limit: RateLimit,
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub last_refill: chrono::DateTime<chrono::Utc>,
    pub tokens: u32,
}

impl RateLimit {
    pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            requests_per_second,
            burst_size,
            last_refill: chrono::Utc::now(),
            tokens: burst_size,
        }
    }

    pub fn try_consume(&mut self) -> bool {
        let now = chrono::Utc::now();
        let time_passed = (now - self.last_refill).num_seconds() as u32;

        // 补充令牌
        let tokens_to_add = time_passed * self.requests_per_second;
        self.tokens = (self.tokens + tokens_to_add).min(self.burst_size);
        self.last_refill = now;

        // 尝试消费令牌
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }
}

impl RateLimitingMiddleware {
    pub fn new(default_requests_per_second: u32, default_burst_size: u32) -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            default_limit: RateLimit::new(default_requests_per_second, default_burst_size),
        }
    }

    async fn check_rate_limit(&self, user_id: Uuid) -> Result<(), EventError> {
        let key = user_id.to_string();
        let mut limits = self.limits.write().await;

        let rate_limit = limits
            .entry(key)
            .or_insert_with(|| self.default_limit.clone());

        if rate_limit.try_consume() {
            Ok(())
        } else {
            Err(EventError::ResourceExhausted(
                "Rate limit exceeded".to_string(),
            ))
        }
    }
}

#[async_trait]
impl EventMiddleware for RateLimitingMiddleware {
    fn name(&self) -> &'static str {
        "rate_limiting"
    }

    fn priority(&self) -> u32 {
        15 // 在认证之后，授权之前
    }

    async fn before_handle(
        &self,
        _event: &dyn Event,
        context: &mut EventContext,
    ) -> Result<(), EventError> {
        if let Some(user_id) = context.user_id() {
            self.check_rate_limit(user_id).await?;
        }
        Ok(())
    }

    async fn after_handle(
        &self,
        _event: &dyn Event,
        _context: &EventContext,
        _result: &EventResult,
    ) -> Result<(), EventError> {
        Ok(())
    }

    fn description(&self) -> &'static str {
        "Rate limits event processing per user"
    }
}

/// 创建默认中间件链
pub fn create_default_middleware_chain() -> MiddlewareChain {
    let mut chain = MiddlewareChain::new();

    chain.add(Arc::new(AuthenticationMiddleware));
    chain.add(Arc::new(RateLimitingMiddleware::new(100, 10))); // 每秒100请求，突发10
    chain.add(Arc::new(AuthorizationMiddleware::new()));
    chain.add(Arc::new(LoggingMiddleware::new(LogLevel::Info)));
    chain.add(Arc::new(PerformanceMiddleware::new(1000))); // 1秒慢查询阈值
    chain.add(Arc::new(CacheMiddleware::new(300))); // 5分钟缓存

    chain
}

// 为Event trait添加缓存相关方法的扩展
pub trait EventCacheExt: Event {
    fn is_cacheable(&self) -> bool {
        false // 默认不可缓存
    }

    fn cache_key_params(&self) -> Vec<String> {
        vec![]
    }
}

impl<T: Event> EventCacheExt for T {}

// 为Event trait添加权限相关方法的扩展
pub trait EventPermissionExt: Event {
    fn required_permissions(&self) -> Vec<String> {
        vec![]
    }

    fn user_id(&self) -> Option<Uuid> {
        None
    }
}

impl<T: Event> EventPermissionExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::events::{ConnectionAction, EventBuilder};

    #[tokio::test]
    async fn test_middleware_chain() {
        let mut chain = MiddlewareChain::new();
        chain.add(Arc::new(LoggingMiddleware::new(LogLevel::Info)));
        chain.add(Arc::new(PerformanceMiddleware::new(1000)));

        let event = EventBuilder::connection_event(
            ConnectionAction::Connect,
            Uuid::new_v4(),
            "test-connection".to_string(),
        );
        let mut context = EventContext::new();

        let result = chain.execute_before(&event, &mut context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let middleware = RateLimitingMiddleware::new(1, 1); // 每秒1请求
        let user_id = Uuid::new_v4();

        // 第一个请求应该成功
        assert!(middleware.check_rate_limit(user_id).await.is_ok());

        // 第二个请求应该被限制
        assert!(middleware.check_rate_limit(user_id).await.is_err());
    }

    #[tokio::test]
    async fn test_cache_middleware() {
        let middleware = CacheMiddleware::new(60);
        // 缓存测试需要实现可缓存的事件类型
        // 这里只测试中间件创建
        assert_eq!(middleware.name(), "cache");
    }
}
