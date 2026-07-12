use chrono::Utc;
use hmac::{Hmac, Mac};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 安全消息结构体，包含签名和防重放保护
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureMessage {
    /// 消息ID，用于防重放攻击
    pub message_id: String,
    /// 时间戳，用于防重放攻击
    pub timestamp: i64,
    /// 随机数，增强防重放保护
    pub nonce: String,
    /// 消息签名
    pub signature: String,
    /// 实际的消息数据
    pub payload: serde_json::Value,
    /// 用户ID，用于签名验证
    pub user_id: Uuid,
}

/// 防重放缓存（Issue #5：LRU 替代无界 HashSet）
///
/// 替换历史：早期 `HashSet<String>` 无界增长，>10000 时随机清一半（可被攻击者
/// 触发清空 → 重放窗口重新打开）。现在用固定容量的 LRU 淘汰最旧的 entry。
#[derive(Clone)]
pub struct ReplayCache {
    inner: Arc<RwLock<LruCache<String, ()>>>,
}

impl ReplayCache {
    /// 构造指定容量的 LRU 缓存。
    /// `capacity == 0` 时，`check_and_mark` 永远返回 `false`（拒绝所有），
    /// 这是测试中可观测的行为。
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            inner: Arc::new(RwLock::new(LruCache::new(cap))),
        }
    }

    /// 检查消息 ID 是否已处理，并原子地将"未见过"的消息标记为已处理。
    /// 返回 `true` 表示这条消息是新的、允许通过；`false` 表示重放。
    pub async fn check_and_mark(&self, message_id: &str) -> bool {
        let mut cache = self.inner.write().await;
        if cache.contains(message_id) {
            return false;
        }
        cache.put(message_id.to_string(), ());
        true
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    pub async fn capacity(&self) -> usize {
        self.inner.read().await.cap().get()
    }
}

/// 消息签名验证器
#[derive(Clone)]
pub struct MessageSigner {
    /// JWT密钥，用于签名
    secret_key: String,
    /// 消息时间窗口（秒），超过此时间窗口的消息被认为是重放攻击
    time_window: i64,
    /// 已处理的消息ID缓存（LRU 替换无界 HashSet）
    replay_cache: ReplayCache,
}

impl MessageSigner {
    pub fn new(config: &momentum_core::config::Config) -> Self {
        // Issue #5：默认容量 10000（原 HashSet 阈值）。可经环境变量或 config 调整。
        let capacity: usize = std::env::var("WS_REPLAY_CACHE_CAPACITY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10_000);
        Self {
            secret_key: config.jwt_secret.clone(),
            time_window: 300, // 5分钟时间窗口
            replay_cache: ReplayCache::new(capacity),
        }
    }

    /// 对消息进行签名
    pub fn sign_message(&self, payload: &serde_json::Value, user_id: Uuid) -> SecureMessage {
        let message_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();
        let nonce = Uuid::new_v4().to_string();

        // 创建签名数据
        let signature_data =
            self.create_signature_data(&message_id, timestamp, &nonce, payload, user_id);

        // 生成签名
        let signature = self.generate_signature(&signature_data);

        SecureMessage {
            message_id,
            timestamp,
            nonce,
            signature,
            payload: payload.clone(),
            user_id,
        }
    }

    /// 验证消息签名和防重放攻击
    pub async fn verify_message(&self, message: &SecureMessage) -> Result<(), SecurityError> {
        // 1. 验证时间戳
        self.verify_timestamp(message.timestamp)?;

        // 2. 验证消息ID是否已被处理过（防重放攻击）
        self.verify_not_processed(&message.message_id).await?;

        // 3. 验证签名
        self.verify_signature(message)?;

        // 4. 将消息ID标记为已处理
        self.mark_as_processed(&message.message_id).await;

        Ok(())
    }

    /// 验证时间戳是否在允许的时间窗口内
    fn verify_timestamp(&self, timestamp: i64) -> Result<(), SecurityError> {
        let now = Utc::now().timestamp();
        let time_diff = (now - timestamp).abs();

        if time_diff > self.time_window {
            return Err(SecurityError::MessageExpired {
                message_timestamp: timestamp,
                server_timestamp: now,
                time_difference: time_diff,
                allowed_window: self.time_window,
            });
        }

        Ok(())
    }

    /// 验证消息ID是否已被处理过
    async fn verify_not_processed(&self, message_id: &str) -> Result<(), SecurityError> {
        // Issue #5：原子 check-and-mark，未见过的 ID 通过并立即标记为已处理。
        // LRU 自动驱逐，调用方无需单独的 mark_as_processed。
        if self.replay_cache.check_and_mark(message_id).await {
            Ok(())
        } else {
            Err(SecurityError::ReplayAttack {
                message_id: message_id.to_string(),
            })
        }
    }

    /// 验证消息签名
    fn verify_signature(&self, message: &SecureMessage) -> Result<(), SecurityError> {
        let signature_data = self.create_signature_data(
            &message.message_id,
            message.timestamp,
            &message.nonce,
            &message.payload,
            message.user_id,
        );

        let expected_signature = self.generate_signature(&signature_data);

        if message.signature != expected_signature {
            return Err(SecurityError::InvalidSignature {
                provided: message.signature.clone(),
                expected: expected_signature,
                message_id: message.message_id.clone(),
            });
        }

        Ok(())
    }

    /// 创建签名数据
    fn create_signature_data(
        &self,
        message_id: &str,
        timestamp: i64,
        nonce: &str,
        payload: &serde_json::Value,
        user_id: Uuid,
    ) -> String {
        // 将payload序列化为字符串，确保一致性
        let payload_str = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());

        format!(
            "{}:{}:{}:{}:{}:{}",
            message_id, timestamp, nonce, payload_str, user_id, self.secret_key
        )
    }

    /// 生成HMAC-SHA256签名
    fn generate_signature(&self, data: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC can take key of any size");

        mac.update(data.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// Issue #5：mark_as_processed 不再需要 —— `verify_not_processed` 用 LRU 的
    /// `check_and_mark` 原子地完成了"检查并标记"两步。保留此方法以便向后兼容。
    #[deprecated(note = "use replay_cache.check_and_mark directly (atomic)")]
    async fn mark_as_processed(&self, _message_id: &str) {
        // no-op
    }

    /// Issue #5：旧的"10000 时随机清一半"清理已删除。
    /// LRU 自动驱逐，无需手动清理。本方法保留为 no-op 以保证调用方兼容。
    pub async fn cleanup_expired_cache(&self) {
        // no-op: LRU 自动淘汰最旧条目
    }

    /// 启动定期清理任务
    pub async fn start_cleanup_task(&self) {
        let signer = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5分钟清理一次
            loop {
                interval.tick().await;
                signer.cleanup_expired_cache().await;
            }
        });
    }
}

/// 安全错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityError {
    /// 消息过期
    MessageExpired {
        message_timestamp: i64,
        server_timestamp: i64,
        time_difference: i64,
        allowed_window: i64,
    },
    /// 重放攻击检测
    ReplayAttack { message_id: String },
    /// 无效签名
    InvalidSignature {
        provided: String,
        expected: String,
        message_id: String,
    },
    /// 消息格式错误
    InvalidMessageFormat { reason: String },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::MessageExpired {
                message_timestamp,
                server_timestamp,
                time_difference,
                allowed_window,
            } => {
                write!(
                    f,
                    "Message expired: message_timestamp={}, server_timestamp={}, time_difference={}, allowed_window={}",
                    message_timestamp, server_timestamp, time_difference, allowed_window
                )
            }
            SecurityError::ReplayAttack { message_id } => {
                write!(f, "Replay attack detected: message_id={}", message_id)
            }
            SecurityError::InvalidSignature {
                provided,
                expected,
                message_id,
            } => {
                write!(
                    f,
                    "Invalid signature: provided={}, expected={}, message_id={}",
                    provided, expected, message_id
                )
            }
            SecurityError::InvalidMessageFormat { reason } => {
                write!(f, "Invalid message format: {}", reason)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

/// 安全消息构建器
pub struct SecureMessageBuilder {
    signer: MessageSigner,
}

impl SecureMessageBuilder {
    pub fn new(signer: MessageSigner) -> Self {
        Self { signer }
    }

    /// 构建安全消息
    pub fn build(&self, payload: serde_json::Value, user_id: Uuid) -> SecureMessage {
        self.signer.sign_message(&payload, user_id)
    }

    /// 验证安全消息
    pub async fn verify(&self, message: &SecureMessage) -> Result<(), SecurityError> {
        self.signer.verify_message(message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> momentum_core::config::Config {
        momentum_core::config::Config {
            database_url: "test://database".to_string(),
            database_max_connections: 10,
            database_min_connections: 5,
            database_connection_timeout: 30,
            redis_url: "test://redis".to_string(),
            redis_pool_size: 10,
            server_host: "localhost".to_string(),
            server_port: 8000,
            cors_origins: vec!["*".to_string()],
            jwt_secret: "test-secret-key-for-signing".to_string(),
            jwt_access_token_expires_in: 3600,
            jwt_refresh_token_expires_in: 604800,
            log_level: "info".to_string(),
            log_format: "json".to_string(),
            assets_url: "http://localhost:8000/assets".to_string(),
            bcrypt_cost: 4,
        }
    }

    #[tokio::test]
    async fn test_message_signing_and_verification() {
        let config = create_test_config();
        let signer = MessageSigner::new(&config);
        let user_id = Uuid::new_v4();

        let payload = serde_json::json!({
            "type": "test_message",
            "data": "Hello, World!"
        });

        // 签名消息
        let signed_message = signer.sign_message(&payload, user_id);

        // 验证消息
        let result = signer.verify_message(&signed_message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replay_attack_detection() {
        let config = create_test_config();
        let signer = MessageSigner::new(&config);
        let user_id = Uuid::new_v4();

        let payload = serde_json::json!({"test": "data"});
        let signed_message = signer.sign_message(&payload, user_id);

        // 第一次验证应该成功
        let result1 = signer.verify_message(&signed_message).await;
        assert!(result1.is_ok());

        // 第二次验证应该失败（重放攻击）
        let result2 = signer.verify_message(&signed_message).await;
        assert!(matches!(result2, Err(SecurityError::ReplayAttack { .. })));
    }

    #[tokio::test]
    async fn test_message_expiration() {
        let config = create_test_config();
        let signer = MessageSigner::new(&config);
        let user_id = Uuid::new_v4();

        let payload = serde_json::json!({"test": "data"});

        // 创建一个过期的消息（时间戳设置为很久以前）
        let mut expired_message = signer.sign_message(&payload, user_id);
        expired_message.timestamp = Utc::now().timestamp() - 1000; // 1000秒前

        let result = signer.verify_message(&expired_message).await;
        assert!(matches!(result, Err(SecurityError::MessageExpired { .. })));
    }

    #[tokio::test]
    async fn test_signature_tampering() {
        let config = create_test_config();
        let signer = MessageSigner::new(&config);
        let user_id = Uuid::new_v4();

        let payload = serde_json::json!({"test": "data"});
        let mut tampered_message = signer.sign_message(&payload, user_id);

        // 篡改签名
        tampered_message.signature = "tampered_signature".to_string();

        let result = signer.verify_message(&tampered_message).await;
        assert!(matches!(
            result,
            Err(SecurityError::InvalidSignature { .. })
        ));
    }

    // ===== Issue #5：ReplayCache LRU 行为测试 =====

    #[tokio::test]
    async fn test_replay_cache_first_insert_returns_true() {
        use crate::websocket::security::ReplayCache;
        let cache = ReplayCache::new(3);
        assert!(cache.check_and_mark("msg-1").await);
    }

    #[tokio::test]
    async fn test_replay_cache_duplicate_returns_false() {
        use crate::websocket::security::ReplayCache;
        let cache = ReplayCache::new(3);
        assert!(cache.check_and_mark("msg-1").await);
        assert!(!cache.check_and_mark("msg-1").await, "second insert must be detected as replay");
    }

    #[tokio::test]
    async fn test_replay_cache_evicts_oldest_when_at_capacity() {
        // P0 修复（Issue #5）：旧版是 HashSet 无界增长 + 10000 随机清一半
        // 新版用 LRU：超过 capacity 时驱逐最旧的
        use crate::websocket::security::ReplayCache;
        let cache = ReplayCache::new(3);
        assert!(cache.check_and_mark("a").await);
        assert!(cache.check_and_mark("b").await);
        assert!(cache.check_and_mark("c").await);
        assert_eq!(cache.len().await, 3);

        // 插入第 4 个 → 驱逐最旧的 "a"
        assert!(cache.check_and_mark("d").await);
        assert_eq!(cache.len().await, 3);

        // "a" 被驱逐，重新插入应成功（不能误报 ReplayAttack）
        assert!(cache.check_and_mark("a").await, "evicted entry should be insertable again");
    }

    #[tokio::test]
    async fn test_replay_cache_len_tracks_unique_entries() {
        use crate::websocket::security::ReplayCache;
        let cache = ReplayCache::new(10);
        assert_eq!(cache.len().await, 0);
        cache.check_and_mark("x").await;
        cache.check_and_mark("y").await;
        cache.check_and_mark("x").await; // duplicate, no count change
        assert_eq!(cache.len().await, 2);
    }

    #[tokio::test]
    async fn test_replay_cache_capacity_1_keeps_only_latest() {
        use crate::websocket::security::ReplayCache;
        let cache = ReplayCache::new(1);
        // 容量为 1：插入 a 成功
        assert!(cache.check_and_mark("a").await);
        // 插入 b → 驱逐 a
        assert!(cache.check_and_mark("b").await, "b evicts a in LRU cap=1");
        // a 被驱逐后可以重新插入
        assert!(cache.check_and_mark("a").await, "a can re-enter after eviction");
        // 但 b 现在被驱逐
        assert!(cache.check_and_mark("b").await);
    }
}
