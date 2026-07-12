# WebSocket 功能文档

本文档介绍了Momentum后端项目中WebSocket功能的实现、使用方法和测试。

## 功能概述

WebSocket功能提供了实时通信能力，支持以下特性：

- **JWT认证验证** - 使用现有的JWT认证系统保护WebSocket连接
- **实时消息广播** - 支持向所有在线用户广播消息
- **点对点消息** - 支持向特定用户发送消息
- **连接管理** - 自动管理连接生命周期，包括连接清理
- **心跳检测** - 支持ping/pong机制保持连接活跃
- **多种消息类型** - 支持文本、通知、系统消息等多种类型

## WebSocket端点

### 主要端点

- **WebSocket连接**: `ws://localhost:8000/ws?token=<JWT_TOKEN>`
- **在线用户列表**: `GET /ws/online`
- **连接统计**: `GET /ws/stats` 
- **发送消息**: `POST /ws/send`
- **广播消息**: `POST /ws/broadcast`
- **清理连接**: `POST /ws/cleanup`

## 认证方式

WebSocket连接必须通过JWT token进行认证：

```javascript
// 连接WebSocket
const token = "your_jwt_token_here";
const ws = new WebSocket(`ws://localhost:8000/ws?token=${token}`);
```

### JWT Token要求

token必须包含以下claims：
- `sub`: 用户ID (UUID)
- `username`: 用户名
- `email`: 用户邮箱
- `exp`: 过期时间
- `iat`: 签发时间
- `jti`: JWT ID

## 消息格式

### 标准消息结构

```json
{
  "id": "message-uuid",
  "message_type": "text|notification|system_message|ping|pong|user_joined|user_left|error",
  "data": {
    "content": "消息内容",
    "additional_field": "其他数据"
  },
  "timestamp": "2024-01-01T00:00:00Z",
  "from_user_id": "sender-uuid-optional",
  "to_user_id": "recipient-uuid-optional"
}
```

### 消息类型

1. **text** - 普通文本消息
2. **notification** - 通知消息
3. **system_message** - 系统消息
4. **ping** - 心跳检测请求
5. **pong** - 心跳检测响应
6. **user_joined** - 用户加入通知
7. **user_left** - 用户离开通知
8. **error** - 错误消息

## 客户端使用示例

### JavaScript/WebSocket API

```javascript
const token = "your_jwt_token";
const ws = new WebSocket(`ws://localhost:8000/ws?token=${token}`);

// 连接成功
ws.onopen = function(event) {
    console.log("WebSocket连接已建立");
    
    // 发送ping消息
    ws.send(JSON.stringify({
        id: crypto.randomUUID(),
        message_type: "ping",
        data: {},
        timestamp: new Date().toISOString()
    }));
};

// 接收消息
ws.onmessage = function(event) {
    const message = JSON.parse(event.data);
    console.log("收到消息:", message);
    
    switch(message.message_type) {
        case "text":
            console.log("文本消息:", message.data.content);
            break;
        case "notification":
            console.log("通知:", message.data);
            break;
        case "pong":
            console.log("收到pong响应");
            break;
        case "user_joined":
            console.log("用户加入:", message.data.username);
            break;
        case "user_left":
            console.log("用户离开:", message.data.username);
            break;
    }
};

// 发送文本消息
function sendMessage(content) {
    ws.send(JSON.stringify({
        id: crypto.randomUUID(),
        message_type: "text",
        data: {
            content: content
        },
        timestamp: new Date().toISOString()
    }));
}

// 连接关闭
ws.onclose = function(event) {
    console.log("WebSocket连接已关闭");
};

// 连接错误
ws.onerror = function(error) {
    console.error("WebSocket错误:", error);
};
```

## HTTP API端点

### 获取在线用户

```bash
curl -X GET http://localhost:8000/ws/online
```

响应:
```json
{
  "count": 5,
  "users": [
    {
      "user_id": "user-uuid",
      "username": "username",
      "connected_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

### 获取连接统计

```bash
curl -X GET http://localhost:8000/ws/stats
```

响应:
```json
{
  "total_connections": 10,
  "unique_users": 8,
  "server_uptime": 3600
}
```

### 发送消息给特定用户

```bash
curl -X POST http://localhost:8000/ws/send \
  -H "Content-Type: application/json" \
  -d '{
    "to_user_id": "target-user-uuid",
    "message_type": "notification",
    "data": {
      "content": "你有新的通知",
      "type": "info"
    }
  }'
```

### 广播消息

```bash
curl -X POST http://localhost:8000/ws/broadcast \
  -H "Content-Type: application/json" \
  -d '{
    "message_type": "system_message",
    "data": {
      "content": "系统维护通知",
      "priority": "high"
    }
  }'
```

## 测试

### 运行基础测试

```bash
# 运行WebSocket单元测试
cargo test websocket

# 运行集成测试（需要服务器运行）
cargo test --test integration_tests -- --ignored
```

### 运行压力测试

项目包含一个专门的压力测试工具：

```bash
# 编译压力测试工具
cargo build --bin websocket_stress_test

# 运行所有压力测试
./target/debug/websocket_stress_test --test-type all

# 运行连接风暴测试
./target/debug/websocket_stress_test --test-type storm --connections 200

# 运行消息吞吐量测试
./target/debug/websocket_stress_test --test-type throughput --connections 50 --messages 20

# 运行持续负载测试
./target/debug/websocket_stress_test --test-type sustained --duration 120 --connections 30
```

### 压力测试参数

- `--url`: WebSocket服务器URL (默认: ws://127.0.0.1:8000/ws)
- `--connections`: 并发连接数 (默认: 100)
- `--messages`: 每个连接的消息数 (默认: 10)
- `--duration`: 测试持续时间(秒) (默认: 60)
- `--max-concurrent`: 最大并发连接数 (默认: 50)
- `--message-interval`: 消息发送间隔(毫秒) (默认: 100)

### 测试类型

1. **storm** - 连接风暴测试，快速建立大量连接
2. **throughput** - 消息吞吐量测试，测试消息处理能力
3. **sustained** - 持续负载测试，长时间维持连接和消息交换
4. **all** - 运行所有测试类型

## 部署注意事项

### 环境变量

确保设置以下环境变量：

```bash
# JWT密钥（必须与认证系统一致）
JWT_SECRET=your_jwt_secret_key

# 数据库连接
DATABASE_URL=postgresql://user:password@localhost/momentum

# Redis连接（如果使用）
REDIS_URL=redis://localhost:6379
```

### 性能调优

1. **连接池大小**: 根据预期并发连接数调整数据库连接池
2. **内存限制**: 监控内存使用，特别是大量连接时
3. **清理间隔**: 定期清理过期连接，默认5分钟间隔

### 监控指标

建议监控以下指标：

- 活跃WebSocket连接数
- 消息发送/接收速率
- 连接建立/断开速率
- 内存和CPU使用率
- 数据库连接池状态

## 故障排除

### 常见问题

1. **连接失败**
   - 检查JWT token是否有效
   - 确认用户是否存在且处于活跃状态
   - 检查服务器日志

2. **消息不能发送**
   - 检查消息格式是否正确
   - 确认连接是否仍然活跃
   - 查看WebSocket连接状态

3. **性能问题**
   - 检查数据库连接池配置
   - 监控内存使用情况
   - 考虑增加清理频率

### 调试日志

启用详细日志：

```bash
RUST_LOG=debug cargo run
```

## 架构设计

### 核心组件

1. **WebSocketManager** - 连接管理器，负责连接生命周期
2. **WebSocketAuth** - 认证模块，处理JWT验证
3. **WebSocketHandler** - 请求处理器，处理HTTP升级和消息路由
4. **MessageType** - 消息类型定义

### 设计特点

- **线程安全**: 使用Arc和异步锁确保并发安全
- **内存效率**: 连接信息存储优化，支持大量并发
- **扩展性**: 模块化设计，易于添加新功能
- **可靠性**: 自动错误处理和连接清理

## 开发指南

### 添加新消息类型

1. 在`MessageType`枚举中添加新类型
2. 更新消息处理逻辑
3. 添加相应的测试用例

### 扩展认证机制

1. 修改`WebSocketAuth`模块
2. 更新JWT claims验证逻辑
3. 测试新的认证流程

### 性能优化

1. 使用异步处理避免阻塞
2. 合理设置连接池大小
3. 定期进行性能测试