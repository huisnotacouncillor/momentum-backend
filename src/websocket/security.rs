use crate::config::Config;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use hmac::{Hmac, Mac};
use std::collections::HashSet;
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

/// 消息签名验证器
#[derive(Clone)]
pub struct MessageSigner {
    /// JWT密钥，用于签名
    secret_key: String,
    /// 消息时间窗口（秒），超过此时间窗口的消息被认为是重放攻击
    time_window: i64,
    /// 已处理的消息ID缓存，用于防重放攻击
    processed_messages: Arc<RwLock<HashSet<String>>>,
    /// 缓存过期时间（秒）
    #[allow(dead_code)]
    cache_expiration: i64,
}

impl MessageSigner {
    pub fn new(config: &Config) -> Self {
        Self {
            secret_key: config.jwt_secret.clone(),
            time_window: 300, // 5分钟时间窗口
            processed_messages: Arc::new(RwLock::new(HashSet::new())),
            cache_expiration: 3600, // 1小时缓存过期
        }
    }

    /// 对消息进行签名
    pub fn sign_message(&self, payload: &serde_json::Value, user_id: Uuid) -> SecureMessage {
        let message_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();
        let nonce = Uuid::new_v4().to_string();

        // 创建签名数据
        let signature_data = self.create_signature_data(&message_id, timestamp, &nonce, payload, user_id);

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
        let processed = self.processed_messages.read().await;
        if processed.contains(message_id) {
            return Err(SecurityError::ReplayAttack {
                message_id: message_id.to_string(),
            });
        }
        Ok(())
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
            message_id,
            timestamp,
            nonce,
            payload_str,
            user_id,
            self.secret_key
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

    /// 将消息ID标记为已处理
    async fn mark_as_processed(&self, message_id: &str) {
        let mut processed = self.processed_messages.write().await;
        processed.insert(message_id.to_string());
    }

    /// 清理过期的消息ID缓存
    pub async fn cleanup_expired_cache(&self) {
        // 这里可以添加基于时间戳的清理逻辑
        // 由于我们使用的是HashSet，这里暂时保留所有记录
        // 在实际生产环境中，可能需要实现基于时间戳的清理
        let mut processed = self.processed_messages.write().await;
        if processed.len() > 10000 {
            // 如果缓存太大，清理一半
            let to_remove: Vec<String> = processed.iter().take(processed.len() / 2).cloned().collect();
            for id in to_remove {
                processed.remove(&id);
            }
        }
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
    ReplayAttack {
        message_id: String,
    },
    /// 无效签名
    InvalidSignature {
        provided: String,
        expected: String,
        message_id: String,
    },
    /// 消息格式错误
    InvalidMessageFormat {
        reason: String,
    },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::MessageExpired {
                message_timestamp,
                server_timestamp,
                time_difference,
                allowed_window
            } => {
                write!(
                    f,
                    "Message expired: message_timestamp={}, server_timestamp={}, time_difference={}, allowed_window={}",
                    message_timestamp,
                    server_timestamp,
                    time_difference,
                    allowed_window
                )
            }
            SecurityError::ReplayAttack { message_id } => {
                write!(f, "Replay attack detected: message_id={}", message_id)
            }
            SecurityError::InvalidSignature { provided, expected, message_id } => {
                write!(
                    f,
                    "Invalid signature: provided={}, expected={}, message_id={}",
                    provided,
                    expected,
                    message_id
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
    use crate::config::Config;

    fn create_test_config() -> Config {
        Config {
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
        assert!(matches!(result, Err(SecurityError::InvalidSignature { .. })));
    }
}
