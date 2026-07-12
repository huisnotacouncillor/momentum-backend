# WebSocket 安全

> 对应代码：`momentum_api/src/websocket/security.rs`、`momentum_api/src/websocket/auth.rs`

---

## 1. 三大机制

| 机制 | 实现 | 默认值 |
|---|---|---|
| **JWT 认证** | `websocket/auth.rs` | token 必须有效且未过期 |
| **HMAC 消息签名** | `security.rs::MessageSigner` | `JWT_SECRET` 作为密钥 |
| **时间窗口防重放** | `MessageSigner::verify_message` | 5 分钟 |

---

## 2. SecureMessage 结构

```rust
pub struct SecureMessage {
    pub message_id: String,           // 防重放
    pub timestamp: i64,               // 防重放
    pub nonce: String,                // 增强随机性
    pub signature: String,            // HMAC-SHA256
    pub payload: serde_json::Value,   // 命令体
    pub user_id: Uuid,                // 签名绑定用户
}
```

签名数据：
```
{message_id}:{timestamp}:{nonce}:{payload_json}:{user_id}:{secret_key}
```

---

## 3. 客户端使用

```rust
use rust_backend::websocket::commands::WebSocketCommand;

let command = WebSocketCommand::CreateLabel {
    idempotency_key: "unique-key-123".to_string(),
    data: CreateLabelCommand { /* ... */ },
};

let payload = serde_json::to_value(&command)?;
let secure_message = message_signer.sign_message(&payload, user_id);
websocket.send(serde_json::to_string(&secure_message)?).await?;
```

## 4. 服务端验证

```rust
let secure_message: SecureMessage = serde_json::from_str(&msg)?;
let response = command_handler
    .handle_secure_command(secure_message, &user)
    .await;
```

验证失败 → 返回 `WebSocketCommandResponse { success: false, error: { code: "SECURITY_ERROR" } }`

---

## 5. 错误类型

```rust
pub enum SecurityError {
    MessageExpired {
        message_timestamp: i64,
        server_timestamp: i64,
        time_difference: i64,
        allowed_window: i64,
    },
    ReplayAttack { message_id: String },
    InvalidSignature {
        provided: String,
        expected: String,
        message_id: String,
    },
    InvalidMessageFormat { reason: String },
}
```

---

## 6. 性能配置

```rust
impl MessageSigner {
    pub fn new(config: &Config) -> Self {
        Self {
            secret_key: config.jwt_secret.clone(),
            time_window: 300,        // 5 分钟
            cache_expiration: 3600,  // 1 小时
        }
    }
}
```

缓存限制 10000 条，超出时随机驱逐 50%（⚠️ 见已知问题）。

---

## 7. 已知问题

来自 `docs/architecture/ARCHITECTURE_REVIEW.md`：

### 🔴 JWT 放在 URL query（P2）

```javascript
const ws = new WebSocket(`ws://host/ws?token=${jwt}`);
```

风险：
- 进入 nginx access log
- 进入浏览器历史
- 进入 Proxy/CDN 日志

**修复建议**：改用 `Sec-WebSocket-Protocol` 子协议头传递 token。

### 🔴 重放缓存无界（P1）

`security.rs::processed_messages` 仅当 > 10000 时随机清一半：
- 攻击者可注入 10k+ 消息触发清空，重放窗口重新打开
- 缓存大小不可预测

**修复建议**：用 LRU 替换随机清空；或基于 timestamp 严格过期。

### 🟡 默认密钥回退

如果 `JWT_SECRET` 未设置，`AuthConfig::default()` 回退到 `"your-secret-key"`。生产环境**必须**显式设置。

---

## 8. 最佳实践

1. **密钥**：≥ 32 字符，定期轮换
2. **时间窗口**：5-10 分钟（按网络延迟调整）
3. **监控**：记录安全验证失败事件 + 重放尝试
4. **错误处理**：客户端仅返回通用错误，详细信息记服务端日志

---

## 9. 测试

```bash
cargo test websocket::security
cargo run --example websocket_security_demo
```

---

**最后更新**：2026-07-12