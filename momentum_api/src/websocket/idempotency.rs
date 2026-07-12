//! WS 命令幂等缓存（Issue #7）
//!
//! 替换历史：之前 `commands/handler.rs:555` 直接 `let idempotency_key =
//! \"disabled\".to_string();`，让 `RequestContext.idempotency_key` 形同虚设，
//! 重试无去重保护。
//!
//! 新设计：
//! - 每次 WS 命令处理前计算 idempotency_key（基于 user + workspace +
//!   command payload 的 hash）
//! - 先查 cache：命中则返回缓存响应，不重跑
//! - 未命中：执行命令，把响应写进 cache，再返回
//! - 同 key + 同 user 才视为幂等；不同 user / 不同 key 视为新调用

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// 一条缓存的执行记录
#[derive(Debug, Clone)]
pub struct CachedExecution {
    /// 序列化后的响应 JSON
    pub response_json: serde_json::Value,
    /// 缓存写入时间（用于 TTL）
    pub stored_at: Instant,
}

impl CachedExecution {
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.stored_at.elapsed() > ttl
    }
}

/// 复合 key：用户 + 命令类型 + 关键 payload 派生 hash
/// （不能只用 hash，会有 userA 干扰 userB 的风险）
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub user_id: Uuid,
    pub command_type: String,
    pub payload_hash: String,
}

impl CacheKey {
    pub fn new(user_id: Uuid, command_type: impl Into<String>, payload_hash: impl Into<String>) -> Self {
        Self {
            user_id,
            command_type: command_type.into(),
            payload_hash: payload_hash.into(),
        }
    }
}

/// 线程安全的幂等缓存
#[derive(Clone)]
pub struct IdempotencyCache {
    inner: Arc<RwLock<LruCache<CacheKey, CachedExecution>>>,
    ttl: Duration,
}

impl IdempotencyCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: Arc::new(RwLock::new(LruCache::new(cap))),
            ttl,
        }
    }

    /// 检查 key 是否已经处理过（且未过期）
    pub async fn get_cached(&self, key: &CacheKey) -> Option<serde_json::Value> {
        let cache = self.inner.read().await;
        cache.peek(key).map(|exec| {
            if exec.is_expired(self.ttl) {
                None
            } else {
                Some(exec.response_json.clone())
            }
        }).flatten()
    }

    /// 写入缓存
    pub async fn store(&self, key: CacheKey, response: serde_json::Value) {
        let mut cache = self.inner.write().await;
        cache.put(
            key,
            CachedExecution {
                response_json: response,
                stored_at: Instant::now(),
            },
        );
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn alice() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-00000000000a").unwrap()
    }

    fn bob() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-00000000000b").unwrap()
    }

    #[tokio::test]
    async fn first_lookup_returns_none() {
        let cache = IdempotencyCache::new(10, Duration::from_secs(60));
        let key = CacheKey::new(alice(), "create_label", "h1");
        assert!(cache.get_cached(&key).await.is_none());
    }

    #[tokio::test]
    async fn second_lookup_returns_cached_response() {
        let cache = IdempotencyCache::new(10, Duration::from_secs(60));
        let key = CacheKey::new(alice(), "create_label", "h1");
        let response = json!({"success": true, "data": {"id": "label-1"}});

        cache.store(key.clone(), response.clone()).await;
        let cached = cache.get_cached(&key).await;
        assert_eq!(cached, Some(response));
    }

    #[tokio::test]
    async fn different_users_get_independent_cached_responses() {
        // 安全：alice 和 bob 用同一 hash，但响应必须独立
        let cache = IdempotencyCache::new(10, Duration::from_secs(60));
        let alice_key = CacheKey::new(alice(), "create_label", "same-hash");
        let bob_key = CacheKey::new(bob(), "create_label", "same-hash");

        cache.store(alice_key.clone(), json!({"for": "alice"})).await;
        cache.store(bob_key.clone(), json!({"for": "bob"})).await;

        assert_eq!(
            cache.get_cached(&alice_key).await,
            Some(json!({"for": "alice"}))
        );
        assert_eq!(
            cache.get_cached(&bob_key).await,
            Some(json!({"for": "bob"}))
        );
    }

    #[tokio::test]
    async fn different_payload_hashes_get_independent_responses() {
        let cache = IdempotencyCache::new(10, Duration::from_secs(60));
        let key1 = CacheKey::new(alice(), "create_label", "h1");
        let key2 = CacheKey::new(alice(), "create_label", "h2");

        cache.store(key1.clone(), json!({"v": 1})).await;
        cache.store(key2.clone(), json!({"v": 2})).await;

        assert_eq!(cache.get_cached(&key1).await, Some(json!({"v": 1})));
        assert_eq!(cache.get_cached(&key2).await, Some(json!({"v": 2})));
    }

    #[tokio::test]
    async fn expired_entries_are_treated_as_missing() {
        // TTL 极短 → 立即过期
        let cache = IdempotencyCache::new(10, Duration::from_millis(1));
        let key = CacheKey::new(alice(), "create_label", "h1");
        cache.store(key.clone(), json!({"x": 1})).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(
            cache.get_cached(&key).await.is_none(),
            "entry should have expired"
        );
    }

    #[tokio::test]
    async fn cache_respects_capacity() {
        let cache = IdempotencyCache::new(2, Duration::from_secs(60));
        let key1 = CacheKey::new(alice(), "a", "1");
        let key2 = CacheKey::new(alice(), "a", "2");
        let key3 = CacheKey::new(alice(), "a", "3");

        cache.store(key1.clone(), json!(1)).await;
        cache.store(key2.clone(), json!(2)).await;
        cache.store(key3.clone(), json!(3)).await;
        assert_eq!(cache.len().await, 2);

        // key1 should be evicted (LRU)
        assert!(cache.get_cached(&key1).await.is_none());
        assert!(cache.get_cached(&key2).await.is_some());
        assert!(cache.get_cached(&key3).await.is_some());
    }
}

/// Issue #7 防退化守门：确保 handler 不再硬编码 `"disabled"` 作为 idempotency_key。
#[cfg(test)]
mod handler_regression_guard_tests {
    /// 编译期解析 ws handler 文件：必须不存在
    /// `let idempotency_key = "disabled"` 这一行。
    #[test]
    fn handler_does_not_hardcode_disabled_idempotency_key() {
        let source = include_str!("commands/handler.rs");
        assert!(
            !source.contains("\"disabled\".to_string()"),
            "Issue #7 fix: WebSocketCommandHandler must NOT hardcode \
             idempotency_key = \"disabled\". The previous bug made every \
             command look like a replay (or not, depending on dispatch), \
             defeating the entire idempotency guarantee. Use \
             self.generate_idempotency_key(&command, user) instead."
        );
    }
}
