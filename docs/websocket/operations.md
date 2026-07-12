# WebSocket 运维手册

> 覆盖：测试、压力测试、监控、部署、故障排查

---

## 1. 测试

### 单元测试

```bash
cargo test websocket
cargo test --lib websocket::commands
```

### 集成测试

```bash
# 需要先启动后端服务
cargo test --test integration_tests -- --ignored
```

---

## 2. 压力测试

工具：`momentum_api/src/bin/websocket_stress_test.rs`

```bash
# 编译
cargo build --bin websocket_stress_test

# 全套
./target/debug/websocket_stress_test --test-type all

# 连接风暴（200 并发瞬时连入）
./target/debug/websocket_stress_test --test-type storm --connections 200

# 消息吞吐（50 连接 × 20 消息）
./target/debug/websocket_stress_test --test-type throughput --connections 50 --messages 20

# 持续负载（30 连接维持 120 秒）
./target/debug/websocket_stress_test --test-type sustained --duration 120 --connections 30
```

### 参数

| 参数 | 默认 | 说明 |
|---|---|---|
| `--url` | `ws://127.0.0.1:8000/ws` | 服务地址 |
| `--connections` | 100 | 并发连接数 |
| `--messages` | 10 | 每连接消息数 |
| `--duration` | 60 | 持续时长（秒） |
| `--max-concurrent` | 50 | 最大并发 |
| `--message-interval` | 100 | 消息间隔（ms） |

### 测试类型

| 类型 | 用途 |
|---|---|
| `storm` | 瞬时大量连接（模拟握手峰值） |
| `throughput` | 消息吞吐上限 |
| `sustained` | 长时间维持（检测内存泄漏） |
| `all` | 全部运行 |

---

## 3. 监控

### 启用指标

参见 `docs/observability/metrics.md`：

```bash
curl http://localhost:8000/metrics | grep ws_
# momentum_ws_connections{state="active"} 23
# momentum_ws_messages_total{direction="in",message_type="command"} 1024
```

### 关键指标

| 指标 | 含义 | 告警阈值建议 |
|---|---|---|
| `momentum_ws_connections{state="active"}` | 当前活跃连接 | > 80% `WS_MAX_CONNECTIONS` 持续 5min |
| `momentum_ws_messages_total` 增长率 | 消息吞吐 | 突变 ±50% 持续 5min |
| `momentum_errors_total{layer="websocket"}` | WS 层错误 | > 1% 总消息 |

### 日志

```bash
# 启用调试
RUST_LOG=debug cargo run

# 仅 WS
RUST_LOG=momentum_api::websocket=debug cargo run
```

---

## 4. 部署配置

### 环境变量

```env
# 必须
JWT_SECRET=<与认证系统一致>

# 推荐
WS_MAX_CONNECTIONS=10000
WS_CONNECTION_TIMEOUT=300
WS_RATE_LIMIT_PER_SECOND=10
WS_RATE_LIMIT_WINDOW=60
WS_CLEANUP_INTERVAL=300
```

### 资源建议

| 并发连接 | CPU | 内存 | DB 连接池 |
|---|---|---|---|
| 1k | 2 核 | 512 MB | 20 |
| 10k | 8 核 | 4 GB | 50 |
| 50k | 32 核 | 16 GB | 100 |

### 反向代理注意

如果走 nginx/cloudfront：

- 启用 WebSocket upgrade（`Upgrade` / `Connection` 头透传）
- 超时 ≥ ping 间隔 × 3（避免 idle 连接被切断）
- ⚠️ JWT 在 URL query → nginx access log 会记录，**生产建议改用 Sec-WebSocket-Protocol 头**

---

## 5. 故障排查

### 连接失败

| 症状 | 排查 |
|---|---|
| 401 立刻断开 | JWT 无效/过期，看 `/auth/profile` 校验 |
| 握手后 1s 内断开 | 中间件 panic，看 stderr 日志 |
| 握手成功但立即 idle timeout | 反代 idle timeout 过短 |

### 消息丢失

| 症状 | 排查 |
|---|---|
| 命令发了无响应 | `request_id` 是否带回？查日志 `request_id = xxx` |
| 事件不推送 | 是否调用 `Subscribe`？topic 是否正确？ |
| 部分连接收不到 | 检查工作区过滤（`ARCHITECTURE_REVIEW.md` §2 漏洞 3） |

### 性能下降

| 症状 | 排查 |
|---|---|
| 消息延迟升高 | 看 `ws_messages_total` 速率、DB 连接池使用率 |
| 内存增长 | `subscription/manager.rs` 是否有未清理的 topic 订阅 |
| CPU 飙升 | 看是否进入 `expect("Failed to get DB connection")` panic 路径 |

### 调试工具

```bash
# 交互式客户端（手动发命令）
cargo run --bin websocket_client

# 启用 trace 级日志
RUST_LOG=momentum_api::websocket=trace cargo run
```

---

## 6. 升级与兼容性

- 当前命令系统向后兼容：新增命令不破坏旧客户端
- 字段新增可选，向后兼容
- ⚠️ 字段删除或语义变更需走 ADR

---

**最后更新**：2026-07-12