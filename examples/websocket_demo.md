# WebSocket 功能演示

本文档演示了 Momentum 后端 WebSocket 功能的各种使用场景和测试方法。

## 概览

我们的 WebSocket 实现提供了以下核心功能：

- 🔐 **JWT 认证保护** - 所有连接必须通过有效的 JWT token 验证
- 💬 **实时消息传递** - 支持文本、通知、系统消息等多种类型
- 📡 **广播系统** - 向所有在线用户或特定用户发送消息
- 💓 **心跳检测** - 自动ping/pong机制保持连接活跃
- 🔧 **连接管理** - 自动清理过期连接，提供在线状态查询

## 快速开始

### 1. 启动服务器

```bash
# 设置环境变量
export JWT_SECRET="your-secret-key"
export DATABASE_URL="postgresql://localhost/momentum_dev"
export REDIS_URL="redis://localhost:6379"

# 启动服务器
cargo run
```

服务器启动后，WebSocket 端点将在 `ws://127.0.0.1:8000/ws` 可用。

### 2. 使用内置客户端工具

我们提供了一个功能完整的 WebSocket 客户端工具用于测试：

```bash
# 构建客户端工具
cargo build --bin websocket_client

# 启动交互式客户端
./target/debug/websocket_client --username "demo_user" --email "demo@example.com"
```

## 演示场景

### 场景1：基础连接和消息发送

```bash
# 启动客户端
./target/debug/websocket_client --username "alice" --email "alice@example.com"
```

在客户端中执行以下命令：

```
websocket> text Hello everyone!
websocket> ping
websocket> notification 这是一条通知消息
```

### 场景2：多用户聊天演示

打开多个终端窗口，每个运行不同的用户：

**终端 1 - Alice:**
```bash
./target/debug/websocket_client --username "alice" --email "alice@example.com"
```

**终端 2 - Bob:**
```bash
./target/debug/websocket_client --username "bob" --email "bob@example.com"
```

**终端 3 - Charlie:**
```bash
./target/debug/websocket_client --username "charlie" --email "charlie@example.com"
```

现在在任意终端发送消息，其他用户都能实时收到：

```
websocket> text 大家好，我是 Alice！
websocket> notification 新用户加入了聊天
```

### 场景3：压力测试演示

测试服务器在高负载下的表现：

```bash
# 轻量级压力测试
./target/debug/websocket_stress_test --connections 50 --messages 10 --test-type throughput

# 连接风暴测试
./target/debug/websocket_stress_test --connections 100 --test-type storm

# 持续负载测试
./target/debug/websocket_stress_test --connections 30 --duration 60 --test-type sustained

# 运行所有测试
./target/debug/websocket_stress_test --test-type all
```

### 场景4：HTTP API 集成演示

除了 WebSocket 连接，我们还提供了 HTTP API 用于管理和监控：

```bash
# 查看在线用户
curl http://127.0.0.1:8000/ws/online | jq

# 查看连接统计
curl http://127.0.0.1:8000/ws/stats | jq

# 广播系统消息
curl -X POST http://127.0.0.1:8000/ws/broadcast \
  -H "Content-Type: application/json" \
  -d '{
    "message_type": "system_message",
    "data": {
      "content": "服务器将在5分钟后维护",
      "priority": "high"
    }
  }'

# 发送给特定用户
curl -X POST http://127.0.0.1:8000/ws/send \
  -H "Content-Type: application/json" \
  -d '{
    "to_user_id": "user-uuid-here",
    "message_type": "notification",
    "data": {
      "content": "你有新的私信",
      "type": "private_message"
    }
  }'
```

## Web 客户端演示

### JavaScript 客户端示例

```html
<!DOCTYPE html>
<html>
<head>
    <title>WebSocket Demo</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        #messages { height: 400px; overflow-y: scroll; border: 1px solid #ccc; padding: 10px; margin: 10px 0; }
        .message { margin: 5px 0; padding: 5px; border-radius: 5px; }
        .text { background-color: #e3f2fd; }
        .notification { background-color: #fff3e0; }
        .system { background-color: #f3e5f5; }
        .user-event { background-color: #e8f5e8; font-style: italic; }
        input, button { margin: 5px; padding: 10px; }
        #messageInput { width: 300px; }
    </style>
</head>
<body>
    <h1>🚀 Momentum WebSocket Demo</h1>
    
    <div id="status">状态: 未连接</div>
    
    <div id="messages"></div>
    
    <div>
        <input type="text" id="messageInput" placeholder="输入消息..." />
        <button onclick="sendMessage()">发送消息</button>
        <button onclick="sendPing()">发送Ping</button>
        <button onclick="sendNotification()">发送通知</button>
    </div>

    <script>
        // 配置 - 在实际使用中，这些应该来自环境变量或配置
        const WS_URL = 'ws://127.0.0.1:8000/ws';
        const JWT_TOKEN = 'your-jwt-token-here'; // 需要有效的JWT token
        
        let ws = null;
        const messagesDiv = document.getElementById('messages');
        const statusDiv = document.getElementById('status');
        const messageInput = document.getElementById('messageInput');

        function updateStatus(message, color = 'black') {
            statusDiv.innerHTML = `状态: <span style="color: ${color}">${message}</span>`;
        }

        function addMessage(content, type = 'text', timestamp = null) {
            const messageDiv = document.createElement('div');
            messageDiv.className = `message ${type}`;
            
            const time = timestamp ? new Date(timestamp).toLocaleTimeString() : new Date().toLocaleTimeString();
            messageDiv.innerHTML = `<strong>[${time}]</strong> ${content}`;
            
            messagesDiv.appendChild(messageDiv);
            messagesDiv.scrollTop = messagesDiv.scrollHeight;
        }

        function connect() {
            try {
                ws = new WebSocket(`${WS_URL}?token=${JWT_TOKEN}`);
                
                ws.onopen = function(event) {
                    updateStatus('已连接', 'green');
                    addMessage('🎉 WebSocket 连接已建立', 'system');
                    
                    // 发送初始ping
                    setTimeout(() => sendPing(), 1000);
                };
                
                ws.onmessage = function(event) {
                    try {
                        const message = JSON.parse(event.data);
                        handleMessage(message);
                    } catch (e) {
                        addMessage(`📨 收到原始消息: ${event.data}`, 'text');
                    }
                };
                
                ws.onclose = function(event) {
                    updateStatus('连接已关闭', 'red');
                    addMessage(`🔐 连接关闭: ${event.reason || '未知原因'}`, 'system');
                };
                
                ws.onerror = function(error) {
                    updateStatus('连接错误', 'red');
                    addMessage(`❌ 连接错误: ${error}`, 'system');
                };
                
            } catch (error) {
                updateStatus('连接失败', 'red');
                addMessage(`❌ 连接失败: ${error.message}`, 'system');
            }
        }

        function handleMessage(message) {
            const { message_type, data, timestamp, from_user_id } = message;
            
            switch (message_type) {
                case 'text':
                    addMessage(`💬 ${data.content}`, 'text', timestamp);
                    break;
                    
                case 'notification':
                    addMessage(`🔔 通知: ${JSON.stringify(data)}`, 'notification', timestamp);
                    break;
                    
                case 'system_message':
                    addMessage(`🖥️ 系统: ${data.content || JSON.stringify(data)}`, 'system', timestamp);
                    break;
                    
                case 'user_joined':
                    addMessage(`👋 ${data.username} 加入了聊天`, 'user-event', timestamp);
                    break;
                    
                case 'user_left':
                    addMessage(`👋 ${data.username} 离开了聊天`, 'user-event', timestamp);
                    break;
                    
                case 'pong':
                    addMessage(`🏓 收到 Pong 响应`, 'system', timestamp);
                    break;
                    
                default:
                    addMessage(`📄 ${message_type}: ${JSON.stringify(data)}`, 'text', timestamp);
            }
        }

        function sendMessage() {
            const content = messageInput.value.trim();
            if (!content || !ws || ws.readyState !== WebSocket.OPEN) return;
            
            const message = {
                id: crypto.randomUUID(),
                message_type: 'text',
                data: { content },
                timestamp: new Date().toISOString()
            };
            
            ws.send(JSON.stringify(message));
            messageInput.value = '';
        }

        function sendPing() {
            if (!ws || ws.readyState !== WebSocket.OPEN) return;
            
            const ping = {
                id: crypto.randomUUID(),
                message_type: 'ping',
                data: { manual: true },
                timestamp: new Date().toISOString()
            };
            
            ws.send(JSON.stringify(ping));
        }

        function sendNotification() {
            if (!ws || ws.readyState !== WebSocket.OPEN) return;
            
            const notification = {
                id: crypto.randomUUID(),
                message_type: 'notification',
                data: {
                    content: '这是一个测试通知',
                    priority: 'normal',
                    source: 'web_demo'
                },
                timestamp: new Date().toISOString()
            };
            
            ws.send(JSON.stringify(notification));
        }

        // 处理回车键发送消息
        messageInput.addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                sendMessage();
            }
        });

        // 页面加载时自动连接
        window.addEventListener('load', function() {
            addMessage('🚀 WebSocket Demo 已加载', 'system');
            addMessage('⚠️  请确保已设置有效的JWT token', 'notification');
            
            // 自动连接（如果有有效token）
            if (JWT_TOKEN && JWT_TOKEN !== 'your-jwt-token-here') {
                connect();
            } else {
                addMessage('❌ 请在代码中设置有效的JWT token后刷新页面', 'system');
            }
        });

        // 定期ping保持连接
        setInterval(() => {
            if (ws && ws.readyState === WebSocket.OPEN) {
                sendPing();
            }
        }, 30000);
    </script>
</body>
</html>
```

## 自动化测试演示

运行我们的完整测试套件：

```bash
# 运行所有WebSocket测试
./scripts/test_websocket.sh

# 跳过性能测试的快速测试
./scripts/test_websocket.sh --skip-performance

# 自定义配置测试
./scripts/test_websocket.sh \
  --server-url http://localhost:8000 \
  --ws-url ws://localhost:8000/ws \
  --jwt-secret your-custom-secret
```

## 实际部署演示

### Docker 部署示例

```dockerfile
# Dockerfile.demo
FROM rust:1.70 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rust_backend /usr/local/bin/
COPY --from=builder /app/examples/websocket_demo.html /usr/share/

EXPOSE 8000

CMD ["rust_backend"]
```

```bash
# 构建和运行
docker build -f Dockerfile.demo -t momentum-websocket-demo .
docker run -p 8000:8000 \
  -e JWT_SECRET=demo-secret \
  -e DATABASE_URL=postgresql://host.docker.internal/momentum \
  momentum-websocket-demo
```

### Nginx 反向代理配置

```nginx
# /etc/nginx/sites-available/momentum-websocket
upstream websocket_backend {
    server 127.0.0.1:8000;
}

server {
    listen 80;
    server_name your-domain.com;

    # HTTP API
    location /api/ {
        proxy_pass http://websocket_backend/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }

    # WebSocket
    location /ws {
        proxy_pass http://websocket_backend/ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        
        # WebSocket specific
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
        proxy_connect_timeout 10;
    }

    # 静态文件演示
    location /demo {
        alias /usr/share/momentum/demo;
        index websocket_demo.html;
    }
}
```

## 监控和调试

### 启用详细日志

```bash
# 启用调试日志
RUST_LOG=debug cargo run

# 只显示WebSocket相关日志
RUST_LOG=rust_backend::websocket=debug cargo run

# 启用所有模块的trace级别日志
RUST_LOG=trace cargo run
```

### 实时监控连接

```bash
# 监控脚本
while true; do
    echo "=== $(date) ==="
    curl -s http://127.0.0.1:8000/ws/stats | jq
    echo
    sleep 5
done
```

### 性能分析

```bash
# 使用perf分析（Linux）
perf record -g cargo run --release
perf report

# 使用valgrind分析内存
valgrind --tool=massif cargo run --release

# Rust内存分析
cargo install cargo-profdata
cargo profdata -- cargo run --release
```

## 常见问题和解决方案

### Q: 连接被拒绝
```
A: 检查JWT token是否有效，确保用户存在且处于活跃状态
   调试命令: ./target/debug/websocket_client --token "your-token" --username "test"
```

### Q: 消息发送失败
```
A: 检查消息格式，确保JSON结构正确
   测试: websocket> json {"id":"test","message_type":"ping","data":{},"timestamp":"2024-01-01T00:00:00Z"}
```

### Q: 连接频繁断开
```
A: 检查网络稳定性，启用自动ping
   解决: ./target/debug/websocket_client --no-auto-ping false
```

### Q: 高负载下性能问题
```
A: 调整数据库连接池大小，增加清理频率
   监控: curl http://127.0.0.1:8000/ws/stats
```

## 总结

本演示展示了 Momentum WebSocket 系统的完整功能：

- ✅ 安全的JWT认证
- ✅ 实时双向通信
- ✅ 多种消息类型支持
- ✅ 自动连接管理
- ✅ 性能监控和测试工具
- ✅ 完整的API生态系统

通过这些示例，你可以：
1. 快速启动和测试WebSocket功能
2. 集成到现有的Web应用中
3. 进行压力测试和性能优化
4. 监控和调试连接问题
5. 部署到生产环境

更多技术细节请参考 `WEBSOCKET_README.md` 文档。