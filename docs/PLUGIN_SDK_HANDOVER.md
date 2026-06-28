# Plugin SDK Handover 文档

> 当前 session 的进度、决策、待办
> 下一个 session 从这里继续

---

## 1. 上下文

用户在做"Momentum 团队协作后端"，目标是把已有 Rust 项目（Axum + Diesel + PG + Redis）升级成 **插件化研发 OS**：
- **核心层**：软件团队通用的 issue/project/cycle/workflow
- **插件层**：行业垂直能力（具身智能是第一个）

战略文档：`docs/PRODUCT_PLAN.md` v3.0、架构文档：`docs/ARCHITECTURE.md` v1.0、SDK 设计：`docs/PLUGIN_SDK_DESIGN.md` v0.1。

**当前 session 实际进展**：把 P0-1 ~ P0-6 全做完了，P0-7/P0-8 还没开始。

---

## 2. 仓库结构

```
momentum-backend/
├── Cargo.toml                # workspace root（4 个 member）
├── proto/plugin.proto        # gRPC 契约（PluginService 8 个 RPC）
├── momentum_core/            # 业务内核 + plugins 模块
│   ├── src/
│   │   ├── plugins/          # ← 本 session 新增
│   │   │   ├── mod.rs
│   │   │   ├── error.rs
│   │   │   ├── manifest.rs        # 14 tests
│   │   │   ├── permission.rs      # 4 tests
│   │   │   ├── audit.rs
│   │   │   ├── json_proto.rs      # 1 test
│   │   │   ├── extension/
│   │   │   │   ├── field.rs       # P0-2
│   │   │   │   ├── agent.rs
│   │   │   │   ├── storage.rs
│   │   │   │   └── event.rs
│   │   │   └── registry/
│   │   │       ├── mod.rs
│   │   │       └── state.rs
│   │   ├── db/
│   │   │   ├── models/{plugin,plugin_installation,issue_field_definition,issue_field_value,plugin_storage,plugin_audit,agent_run}.rs
│   │   │   └── repositories/{plugins,plugin_installations,issue_field_definitions,issue_field_values,plugin_storage,plugin_audit,agent_runs}.rs
│   │   └── ... (原有业务模块，未改动)
│   ├── migrations/2026-06-27-173008_create_plugin_system/{up,down}.sql
│   └── diesel.toml            # 本 session 新增
├── momentum_api/              # HTTP + WebSocket 层（未改）
├── momentum_plugin_host/      # gRPC client + 进程管理
│   ├── src/
│   │   ├── lib.rs            # 导出 proto 模块
│   │   ├── process.rs        # spawn_and_wait 用 TCP 端口
│   │   ├── supervisor.rs     # (plugin_id, workspace_id) → Child
│   │   └── agent_impl.rs     # gRPC client invoke_agent
│   └── build.rs              # 生成 proto
└── plugins/plugin-dummy/     # ← 本 session 新增（独立 workspace member）
    ├── Cargo.toml
    ├── build.rs              # 生成 proto
    ├── plugin.yaml           # Manifest
    └── src/main.rs           # gRPC server
```

---

## 3. 已完成（P0-1 ~ P0-8）

| ID | 模块 | 状态 |
|----|------|------|
| P0-1 | plugins 模块骨架 + Manifest 解析 | ✅ 14 tests pass |
| P0-2 | Field / Agent / Storage / Event 4 个扩展点 | ✅ |
| P0-3 | Registry + 状态机 | ✅ 简版（完整 lifecycle 在 host） |
| P0-4 | DB migration（7 表 + outbox + issues.version） | ✅ 跑通，schema.rs 重生 |
| P0-5 | gRPC proto + plugin_host 实现 | ✅ |
| P0-6 | dummy-plugin binary（独立 crate） | ✅ **实测启动 OK（port 19991）** |
| P0-7 | HTTP 路由 + IssueService wire field_values | ✅ 7 个端点 + field_values 注入 |
| P0-8 | clippy 0 errors + tests pass | ✅ 68 passed / 0 failed / clippy 0 errors |

**测试现状**：`cargo test --workspace` → 68 passed / 0 failed（41 in momentum_api + 27 in momentum_core）
**Clippy**：`cargo clippy --workspace -- -D warnings` → 0 errors

---

## 4. 待办

### P0-8 后续（可选）
- E2E 测试：`cargo test --workspace -- --ignored`（需启动真实服务 + DB + plugin-dummy）
- `state.rs` 加 `plugin_host: Arc<Supervisor>`：✅ 已完成
  - `AppState::new` 签名已更新
  - `main.rs` 在 startup 时构造 `Supervisor::new()`
   - `GET /api/v1/workspaces/:wid/fields` (字段定义)
2. 注册到 `momentum_api/src/routes/mod.rs`
3. `momentum_core/src/db/models/issue.rs::IssueResponse` 加 `field_values: HashMap<String, serde_json::Value>` 字段（P0-4 已加）
4. `momentum_core/src/services/issues_service.rs::IssuesService::get_by_id` 注入 field_values：
   ```rust
   use crate::db::repositories::issue_field_values::IssueFieldValueRepo;
   resp.field_values = IssueFieldValueRepo::list_by_issue(conn, issue.id).unwrap_or_default();
   ```
5. `list` 批量加（避免 N+1）：
   ```rust
   let ids: Vec<Uuid> = query.iter().map(|i| i.id).collect();
   let map = IssueFieldValueRepo::list_by_issues(conn, &ids).unwrap_or_default();
   for issue in &query {
       if let Some(v) = map.get(&issue.id) {
           resp.field_values = v.clone();
       }
   }
   ```
6. `momentum_api/src/state.rs` 加 `plugin_host` 字段（如 `Arc<Supervisor>`）

**估计**：1–2 天

### P0-8：测试 + cargo check 0 errors
- `cargo test --workspace` 全过
- `cargo clippy --workspace -- -D warnings` 跑一遍，修 lint
- 跑通真实 DB 集成测试（`DATABASE_URL=... cargo test --test plugin_field_e2e -- --ignored`）

---

## 5. 关键技术决策（不要重新争论）

1. **gRPC 用 TCP localhost 端口**（v0.1），不是 Unix Domain Socket
   - 原因：tonic 0.12 的 `Server::serve(SocketAddr)` 不支持 unix URI
   - 设计文档里写的是 unix socket，**v0.2 切回去**（用 `serve_with_incoming(TcpListenerStream)` 或自己实现 unix listener）
   - 端口约定：dummy-plugin = 19991（可改 `MOMENTUM_PLUGIN_PORT` env var）
   - **注意**：tonic_build `compile` 已 rename 为 `compile_protos`（clippy 要求）

2. **Plugin Lifecycle 拆在两个 crate**：
   - `momentum_core::plugins::registry::PluginRegistry` 只管内存状态
   - `momentum_plugin_host::Supervisor` 管进程 + child handle
   - 这是 Core 不依赖 host 的解耦设计

3. **prost-types 0.13 没 serde feature**：`from_json_string` / `to_json_string` 不可用
   - 手动实现 JSON ↔ prost Struct/Value 转换（`plugins/plugin-dummy/src/main.rs` 末尾 + `momentum_core/src/plugins/json_proto.rs`）

4. **plugin-dummy 是独立 workspace member**（不是 `[[bin]]`）：
   - 原因：`[[bin]]` 的 build.rs 必须放 root，但每个 binary 用的是 `tonic::include_proto!` 需要各自的 build script
   - 文件位置：`plugins/plugin-dummy/{Cargo.toml, build.rs, plugin.yaml, src/main.rs}`

5. **Issue 表加 `version: i32`**：乐观锁字段，已在 schema + Issue/NewIssue/UpdateIssue/IssueResponse 都加好

6. **schema.rs 自动重生成的坑**：
   - `diesel.toml` 里 `import_types` 不够，需手动把每个 `use uuid::Uuid` 改成 `use diesel::sql_types::Uuid`
   - sql_types enum 结构体需要 `derive(diesel::query_builder::QueryId)` 否则 1399 个错

7. **Diesel ExpressionMethods trait**：`outbox::column.eq(value)` 需要 `use diesel::ExpressionMethods;`

---

## 6. 验证命令速查

```bash
# 编译
cargo check --workspace
cargo build --workspace

# 测试
cargo test --workspace

# 跑 plugin-dummy
MOMENTUM_PLUGIN_PORT=19991 ./target/debug/plugin-dummy

# DB migration
cd momentum_core && DATABASE_URL=postgres://postgres:postgres@localhost:5434/rust-backend diesel migration run --config-file momentum_core/diesel.toml

# 重生 schema
DATABASE_URL=postgres://postgres:postgres@localhost:5434/rust-backend diesel print-schema --config-file momentum_core/diesel.toml > momentum_core/src/schema.rs
```

---

## 7. 注意事项 / 已知问题

- **diesel.toml 写法**：必须用 `[print_schema]` + `[migrations_directory]`，**不要**加 `[database]` 段（diesel CLI 会报 `unknown field database`）
- **plugin_storage 表的 value 列**：是 `JSONB`，Diesel 写入用 `serde_json::Value` 直接传，不要 `serde_json::to_string`
- **plugin_audit.payload**：schema 重生后是 `Nullable<Jsonb>`，Rust 端 insert 时直接传 `Option<&serde_json::Value>`
- **agents.rs 引用 `manifest_check`**：需要 manifest 参数，不要从 DB 查（避免循环依赖）
- **workspace plugins[] 不写 core/plugins**：core 里是只读 manifest/permission/extension/registry，进程管理全在 momentum_plugin_host

---

## 8. 下个 session 第一步

所有 P0 都已完成 ✅。下一步可选：
1. E2E 测试（需真实 DB + `DATABASE_URL`）
2. `state.rs` 加 `plugin_host: Arc<Supervisor>` 字段

---

**文档版本**：2026-06-27
**session 完成度**：P0-8/8 (100%) ✅