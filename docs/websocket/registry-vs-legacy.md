# Registry vs Legacy 双分发问题

> ⚠️ 本文档解释 **当前代码存在的已知技术债**，与 `docs/architecture/ARCHITECTURE_REVIEW.md` / `ARCHITECTURE_ISSUES.md` 一致。

---

## 1. 现象

代码中并存两套 WebSocket 命令分发路径：

| 路径 | 实现位置 | 状态 |
|---|---|---|
| **Legacy**（旧） | `momentum_api/src/websocket/commands/handler.rs`（match 语句硬编码） | 实际生效 |
| **Registry**（新） | `momentum_api/src/websocket/registry/handlers/`（基于 `HandlerRegistry` trait） | **死代码** |

### 证据

`momentum_api/src/websocket/mod.rs:96-133`：

```rust
pub fn create_websocket_state(db: Arc<DbPool>, config: &Config) -> WebSocketState {
    WebSocketState {
        command_handler: WebSocketCommandHandler::new(db.clone(), asset_helper)
            .with_message_signer(message_signer.clone()),
        // ❌ .with_registry() 从未被调用
        // ❌ .with_subscription_manager() 从未被调用
    }
}
```

`with_registry()` 和 `with_subscription_manager()` 方法存在但**从未调用**。

---

## 2. Ping 处理两套

### Legacy

`commands/handler.rs:632`：
```rust
WebSocketCommand::Ping { .. } => Ok(serde_json::json!({"message": "pong"})),
```

### Registry（dead code）

`registry/handlers/ping.rs`：
```rust
pub struct PingHandler;
#[async_trait]
impl CommandHandler for PingHandler {
    fn command_type(&self) -> &'static str { "ping" }
    async fn handle(&self, ctx: RequestContext, payload: Value) -> Result<Value, HandlerError> {
        Ok(json!({
            "ok": true,
            "echo": payload,
            "user_id": ctx.user_id,
            "ts": Utc::now()
        }))
    }
}
```

两套实现返回结构**完全不同**。

---

## 3. Subscribe 处理两套

### Legacy（stub）

`commands/handler.rs:912-920`：
```rust
WebSocketCommand::Subscribe { topics, .. } => self.handle_subscribe(ctx, topics).await,

// stub body
Ok(serde_json::json!({
    "subscribed_topics": topics,
    "message": "Successfully subscribed to topics"
}))
```

**只回成功，不实际订阅**。

### Registry（完整）

`registry/handlers/subscribe.rs`：完整实现 topic 解析、验证、订阅逻辑。

---

## 4. 影响

| 维度 | 影响 |
|---|---|
| **维护** | 两套实现并存，改一处要改两处 |
| **一致性** | Legacy stub 与 Registry 完整实现行为不同 |
| **新人理解成本** | 必须理解双分发 + fallback |
| **隐藏 bug** | Subscribe stub 让"看起来工作了，实际没订阅" |

---

## 5. 修复路线

`docs/architecture/REFACTOR_PLAN.md` P2：

1. **决定保留哪一套**：建议保留 Registry（更可扩展）
2. **迁移所有命令 handler 到 Registry**
3. **删除 Legacy `commands/handler.rs` 中的 match 分发**
4. **在 `mod.rs` 中调用 `.with_registry()`**
5. **删除 `with_subscription_manager`（如果 Registry 已包含订阅）**

预计工作量：2-3 天（涉及 65+ 命令）。

---

## 6. 相关文档

- `docs/architecture/ARCHITECTURE_ISSUES.md` §问题 1
- `docs/architecture/ARCHITECTURE_REVIEW.md` §重大遗漏（勘误 1、勘误 3-5）
- `docs/architecture/REFACTOR_PLAN.md` P2

---

**最后更新**：2026-07-12