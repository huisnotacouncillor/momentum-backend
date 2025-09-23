# WebSocket安全功能实现

本文档描述了WebSocket消息签名和防重放攻击的实现。

## 概述

为了保护WebSocket通信的安全性，我们实现了以下安全机制：

1. **消息签名** - 使用HMAC-SHA256对消息进行签名
2. **防重放攻击** - 使用时间戳和消息ID防止消息重放
3. **消息完整性验证** - 确保消息在传输过程中未被篡改

## 核心组件

### 1. SecureMessage结构体

```rust
pub struct SecureMessage {
    pub message_id: String,        // 消息唯一ID，用于防重放
    pub timestamp: i64,           // 时间戳，用于防重放
    pub nonce: String,            // 随机数，增强安全性
    pub signature: String,        // HMAC-SHA256签名
    pub payload: serde_json::Value, // 实际消息数据
    pub user_id: Uuid,            // 用户ID，用于签名验证
}
```

### 2. MessageSigner

负责消息的签名和验证：

```rust
impl MessageSigner {
    // 对消息进行签名
    pub fn sign_message(&self, payload: &serde_json::Value, user_id: Uuid) -> SecureMessage;

    // 验证消息签名和防重放攻击
    pub async fn verify_message(&self, message: &SecureMessage) -> Result<(), SecurityError>;
}
```

## 安全机制详解

### 1. 消息签名

使用HMAC-SHA256算法对消息进行签名：

```rust
// 签名数据包含：
// message_id:timestamp:nonce:payload:user_id:secret_key
let signature_data = format!(
    "{}:{}:{}:{}:{}:{}",
    message_id, timestamp, nonce, payload_str, user_id, secret_key
);

// 生成HMAC-SHA256签名
let signature = hmac_sha256(signature_data);
```

### 2. 防重放攻击

通过以下机制防止重放攻击：

- **时间戳验证**: 消息时间戳必须在允许的时间窗口内（默认5分钟）
- **消息ID缓存**: 已处理的消息ID会被缓存，重复的消息ID会被拒绝
- **随机数**: 每个消息包含唯一的随机数，增加破解难度

### 3. 消息完整性

通过签名验证确保消息完整性：

- 任何对消息内容的篡改都会导致签名验证失败
- 签名包含消息的所有关键信息，确保数据完整性

## 使用方法

### 1. 初始化

```rust
use rust_backend::websocket::security::MessageSigner;
use rust_backend::config::Config;

// 创建配置
let config = Config::from_env()?;

// 创建消息签名器
let message_signer = MessageSigner::new(&config);
```

### 2. 客户端发送消息

```rust
use rust_backend::websocket::commands::WebSocketCommand;

// 1. 创建命令
let command = WebSocketCommand::CreateLabel {
    idempotency_key: "unique-key-123".to_string(),
    data: CreateLabelCommand {
        name: "重要标签".to_string(),
        color: "#FF0000".to_string(),
        level: LabelLevel::Project,
    },
};

// 2. 序列化为JSON
let payload = serde_json::to_value(&command)?;

// 3. 签名消息
let secure_message = message_signer.sign_message(&payload, user_id);

// 4. 发送到服务器
websocket.send(serde_json::to_string(&secure_message)?).await?;
```

### 3. 服务器处理消息

```rust
use rust_backend::websocket::commands::WebSocketCommandHandler;

// 1. 接收消息
let secure_message: SecureMessage = serde_json::from_str(&message)?;

// 2. 使用命令处理器处理（自动验证签名）
let response = command_handler.handle_secure_command(secure_message, &user).await;

// 3. 发送响应
websocket.send(serde_json::to_string(&response)?).await?;
```

## 配置选项

### 环境变量

```bash
# JWT密钥，用于消息签名（必须设置）
JWT_SECRET=your-super-secret-key-at-least-32-characters-long

# 其他配置保持默认值即可
```

### 可调整参数

```rust
impl MessageSigner {
    pub fn new(config: &Config) -> Self {
        Self {
            secret_key: config.jwt_secret.clone(),
            time_window: 300,        // 5分钟时间窗口
            cache_expiration: 3600,  // 1小时缓存过期
        }
    }
}
```

## 安全错误处理

### SecurityError类型

```rust
pub enum SecurityError {
    // 消息过期
    MessageExpired {
        message_timestamp: i64,
        server_timestamp: i64,
        time_difference: i64,
        allowed_window: i64,
    },
    // 重放攻击
    ReplayAttack { message_id: String },
    // 无效签名
    InvalidSignature {
        provided: String,
        expected: String,
        message_id: String,
    },
    // 消息格式错误
    InvalidMessageFormat { reason: String },
}
```

### 错误响应

```rust
// 安全验证失败时的响应
WebSocketCommandResponse {
    idempotency_key: secure_message.message_id,
    success: false,
    data: None,
    error: Some(WebSocketCommandError {
        code: "SECURITY_ERROR".to_string(),
        message: e.to_string(),
        field: None,
    }),
    timestamp: Utc::now(),
}
```

## 性能考虑

### 1. 缓存管理

- 消息ID缓存会自动清理过期条目
- 定期清理任务每5分钟运行一次
- 缓存大小限制为10000条记录

### 2. 签名性能

- HMAC-SHA256签名生成和验证性能优秀
- 签名验证是异步操作，不会阻塞主线程
- 签名数据大小固定，不影响网络传输

### 3. 内存使用

- 消息ID缓存使用HashSet，内存效率高
- 定期清理防止内存泄漏
- 可配置缓存过期时间

## 最佳实践

### 1. 密钥管理

- 使用强密钥（至少32字符）
- 定期轮换密钥
- 不要在代码中硬编码密钥

### 2. 时间窗口

- 根据网络延迟调整时间窗口
- 生产环境建议5-10分钟
- 监控时间验证失败率

### 3. 监控

- 记录安全验证失败事件
- 监控重放攻击尝试
- 跟踪签名验证性能

### 4. 错误处理

- 不要向客户端暴露详细的错误信息
- 记录详细的服务器端日志
- 实现适当的重试机制

## 测试

运行安全功能测试：

```bash
# 运行单元测试
cargo test websocket::security

# 运行演示程序
cargo run --example websocket_security_demo
```

## 故障排除

### 常见问题

1. **签名验证失败**
   - 检查JWT_SECRET是否正确
   - 确认客户端和服务器时间同步
   - 验证消息格式是否正确

2. **重放攻击误报**
   - 检查网络延迟
   - 调整时间窗口设置
   - 确认客户端没有重复发送消息

3. **性能问题**
   - 监控缓存大小
   - 调整清理频率
   - 检查签名验证延迟

### 调试模式

```rust
// 启用详细日志
tracing_subscriber::fmt()
    .with_env_filter("debug")
    .init();
```

## 未来改进

1. **密钥轮换**: 支持动态密钥轮换
2. **压缩**: 对大型消息进行压缩
3. **加密**: 添加端到端加密支持
4. **审计**: 增强安全事件审计功能
