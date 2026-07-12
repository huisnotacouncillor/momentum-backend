# WebSocket 实时通信

> 对应代码：`momentum_api/src/websocket/`
> 协议：JSON-RPC 风格命令系统 + 事件订阅
> 引入时间：2025-08（v0.1 命令协议） → 2026-07（Registry 重构中）

---

## 📚 文档导航

| 文档 | 内容 |
|---|---|
| **[README.md](./README.md)**（本文件） | 总体概览、连接方式、消息格式、安全模型 |
| **[commands.md](./commands.md)** | 全部 65+ 命令的目录，按实体分类 |
| **[operations.md](./operations.md)** | 压力测试、监控、部署、故障排查 |
| **[security.md](./security.md)** | HMAC 签名、防重放、消息完整性 |
| **[registry-vs-legacy.md](./registry-vs-legacy.md)** | 当前存在的双分发问题（与 ARCHITECTURE_REVIEW.md 一致） |

历史版本（2025-08 写的 6 篇文档）已归档到 `docs/_archive/2025/websocket/`。

---

## 1. 端点

### 主连接

```
ws://{host}:8000/ws?token={JWT}
```

JWT 通过 URL query 传递（⚠️ 见 security.md "已知问题"）。

### HTTP 管理端点

| 端点 | 方法 | 用途 |
|---|---|---|
| `/ws/online` | GET | 在线用户列表 |
| `/ws/stats` | GET | 连接统计 |
| `/ws/send` | POST | 发送消息给特定用户 |
| `/ws/broadcast` | POST | 广播给所有用户 |
| `/ws/cleanup` | POST | 手动清理过期连接 |

---

## 2. 消息格式

### 命令请求（客户端 → 服务端）

```json
{
  "command": "CreateIssue",
  "data": { /* 命令负载 */ },
  "request_id": "req-123"
}
```

### 命令响应（服务端 → 客户端）

成功：
```json
{
  "success": true,
  "request_id": "req-123",
  "data": { /* 响应数据 */ }
}
```

失败：
```json
{
  "success": false,
  "request_id": "req-123",
  "error": {
    "code": "ERROR_CODE",
    "message": "...",
    "field": null,
    "details": null,
    "error_type": "validation|not_found|conflict|forbidden|internal"
  }
}
```

### 事件推送（服务端 → 客户端，订阅模式）

```json
{
  "event": "issue.created",
  "data": { /* 事件负载 */ },
  "timestamp": "2026-07-12T10:00:00Z"
}
```

---

## 3. 客户端最小示例

```javascript
const ws = new WebSocket(`ws://localhost:8000/ws?token=${token}`);

ws.onopen = () => {
  ws.send(JSON.stringify({
    command: "Subscribe",
    data: { topics: ["issues", "workspace:abc-123"] },
    request_id: "sub-1"
  }));
};

ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.event) {
    console.log("event:", msg.event, msg.data);
  } else if (msg.success) {
    console.log("response:", msg.request_id, msg.data);
  } else {
    console.error("error:", msg.error);
  }
};
```

完整命令清单见 [commands.md](./commands.md)。

---

## 4. 消息类型（旧 wire 协议，保留兼容）

> ⚠️ 当前实现**主走命令系统**（command/data 字段）。下面是早期 text/notification 风格的协议，仅用于在线状态广播等系统消息。

| message_type | 用途 |
|---|---|
| `text` | 普通文本消息 |
| `notification` | 通知 |
| `system_message` | 系统通知 |
| `ping` / `pong` | 心跳 |
| `user_joined` / `user_left` | 在线状态变更 |
| `error` | 错误 |

---

## 5. 安全

HMAC-SHA256 消息签名 + 时间戳窗口防重放。详见 [security.md](./security.md)。

**已知问题**：
- JWT 放在 URL query → 进入 nginx/反代日志、浏览器历史
- 消息签名缓存无界增长（10k+ 时随机清一半）
- 注册表（Registry）双分发：详见 [registry-vs-legacy.md](./registry-vs-legacy.md)

---

## 6. 速查表

| 我想... | 看哪篇 |
|---|---|
| 发起命令 | commands.md |
| 订阅事件 | commands.md §Subscribe |
| 排查消息签名失败 | security.md |
| 跑压测 | operations.md |
| 了解实现原理 | architecture/ARCHITECTURE_REVIEW.md |
| 找历史文档 | `docs/_archive/2025/websocket/` |

---

**最后更新**：2026-07-12