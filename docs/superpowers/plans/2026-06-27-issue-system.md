# Issue 系统完整实现计划

> **面向 AI 代理的工作者：** 本计划基于代码库存盘点结果，制定 Linear 风格的 Issue 系统实现路径。
> **参考**：`docs/superpowers/plans/2026-06-27-existing-backend.md` 了解现有代码

---

## 一、盘点结果摘要

### 已有
- `Issue` 模型 + `IssueResponse` DTO
- `IssuesRepo` CRUD（部分）
- `IssuesService` 含 enrichment
- HTTP 端点（get/list/create/update/delete）
- `field_values` 批量注入
- `issue_field_values` 表 + repo

### 缺失 / 有 Bug

| 缺陷 | 严重度 | 说明 |
|------|--------|------|
| WebSocket handlers 全是 stub | **P0** | 无法实时协作 |
| `update_fields` 多字段 bug | **P0** | 链式 `if let Some` 只更新第一个字段 |
| API 返回类型不一致 | **P0** | create/update/list 返回 raw Issue，get_by_id 返回 IssueResponse |
| 无分页 | **P1** | `list_*` 返回 unbounded Vec |
| 无团队级 `issue_number` | **P1** | DB SERIAL，全局自增，不是 ENG-123 格式 |
| 无 DB 层过滤 | **P1** | priority/cycle 等在内存过滤 |
| 无 bulk update/delete | **P2** | 批量操作缺失 |
| 无 `IssueFieldDefinitionRepo` | **P2** | 字段定义无法查询 |
| 无 sort 控制 | **P2** | 固定 `created_at desc` |
| 无关系查询（blocks/blocked_by）| **P3** | 子任务之外无关联建模 |

---

## 二、实现计划

### Phase A：修 Bug + API 一致性（P0）

#### 任务 A1：修复 `update_fields` 多字段 bug

**问题**：`if let Some(x) = field; if let Some(y) = field2` 链式写法只会更新第一个 `Some` 的字段，后续 return 导致提前退出。

**文件：** `momentum_core/src/db/repositories/issues.rs`

- [ ] **步骤 1：查看现有 update_fields 代码**

```rust
// 当前有 bug 的写法
pub fn update_fields(...) -> ... {
    if let Some(title) = changes.title {
        // 只更新 title，return 掉了
        diesel::update(...).set(title: title).execute(conn)?;
        return Ok(updated);
    }
    if let Some(desc) = changes.description {
        // 永远不会执行到
        ...
    }
}
```

- [ ] **步骤 2：重写为正确的 AsChangeset 写法**

Diesel 的 `AsChangeset` 支持忽略 None 字段，用法：

```rust
#[derive(AsChangeset)]
#[diesel(table_name = issues)]
pub struct IssueChangeset {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<String>,
    // ...
}

pub fn update_fields(
    conn: &mut PgConnection,
    id: Uuid,
    changeset: IssueChangeset,
) -> Result<Issue, diesel::result::Error> {
    diesel::update(issues::table.filter(issues::id.eq(id)))
        .set(&changeset)
        .get_result(conn)
}
```

- [ ] **步骤 3：验证 build**

```bash
cargo check --package momentum_core
```

- [ ] **步骤 4：写测试**

```rust
#[test]
fn test_update_multiple_fields() {
    // 创建 issue
    // 同时更新 title + status + priority
    // 验证三个字段都更新了
}
```

- [ ] **Commit**: `fix(repo): resolve update_fields multi-field bug`

---

#### 任务 A2：统一 API 返回类型（IssueResponse）

**问题**：
- `create` / `update` / `list` 返回 raw `Issue`
- 只有 `get_by_id` 返回富媒体 `IssueResponse`

**文件：** `momentum_api/src/routes/issues.rs` + `momentum_core/src/services/issues_service.rs`

- [ ] **步骤 1：`create` 返回 IssueResponse**

```rust
// issues.rs route handler
pub async fn create_issue(...) -> IssueResponse {
    let issue = IssuesService::create(&mut conn, &ctx, &payload)?;
    // 现在补充 enrichment
    IssuesService::enrich_issue(&mut conn, issue)
}
```

在 `IssuesService` 新增 `enrich_issue` 方法：

```rust
impl IssuesService {
    pub fn enrich_issue(
        conn: &mut PgConnection,
        issue: Issue,
    ) -> Result<IssueResponse, AppError> {
        let mut resp = IssueResponse::from(issue);
        // team, project, assignee, labels, field_values, ...
        Self::enrich_basic(&mut conn, &mut resp)?;
        resp.field_values = IssueFieldValueRepo::list_by_issue(conn, resp.id).unwrap_or_default();
        Ok(resp)
    }
}
```

- [ ] **步骤 2：`update` 返回 IssueResponse**

```rust
pub async fn update_issue(...) -> IssueResponse {
    let issue = IssuesService::update(&mut conn, &ctx, issue_id, &payload)?;
    IssuesService::enrich_issue(&mut conn, issue)
}
```

- [ ] **步骤 3：`list` 返回 Vec<IssueResponse>，批量 enrichment**

```rust
pub async fn get_issues(...) -> Vec<IssueResponse> {
    let issues = IssuesService::list(...)?;
    let ids: Vec<Uuid> = issues.iter().map(|i| i.id).collect();
    let field_values_map = IssueFieldValueRepo::list_by_issues(conn, &ids)?;
    issues
        .into_iter()
        .map(|issue| {
            let mut resp = IssueResponse::from(issue);
            resp.field_values = field_values_map.get(&resp.id).cloned().unwrap_or_default();
            enrich_basic(&mut conn, &mut resp)?; // team/project/assignee
            Ok(resp)
        })
        .collect()
}
```

- [ ] **步骤 4：Commit**: `feat(api): unified IssueResponse for all issue endpoints`

---

#### 任务 A3：实现 WebSocket issue handlers

**文件：** `momentum_api/src/websocket/commands/issues.rs`

现有 stub：
```rust
// 全是 todo!() 或 "Issue handlers not yet implemented"
```

- [ ] **步骤 1：查看现有 WS 基础设施模式**

```bash
# 看一个已实现的 command handler 作为参考
cat momentum_api/src/websocket/commands/labels.rs
```

- [ ] **步骤 2：实现 `handle_create_issue`**

```rust
pub async fn handle_create_issue(
    manager: &ConnectionManager,
    ws: &mut WebSocket,
    msg: ClientMessage,
) -> Result<(), WsError> {
    // 1. 解析 payload
    let payload: CreateIssueCommand = msg.payload?;

    // 2. JWT 验证（从 ws 提取 user_id）
    let user_id = ws.user_id.ok_or(WsError::Unauthorized)?;

    // 3. 构建 RequestContext
    let ctx = RequestContext { user_id, workspace_id: ws.workspace_id, idempotency_key: None };

    // 4. 调用 service
    let issue = IssuesService::create(conn, &ctx, &payload.input)
        .map_err(WsError::App)?;

    // 5. 广播给同一 workspace 的所有连接
    manager.broadcast_workspace(ws.workspace_id, ServerMessage {
        type: "issue_created",
        payload: serde_json::to_value(&issue)?,
    }).await;

    // 6. 私消息回复 sender
    ws.send_json(ServerMessage {
        type: "issue_created_ack",
        payload: serde_json::to_value(IssueCreatedAck { id: issue.id })?,
    }).await?;

    Ok(())
}
```

- [ ] **步骤 3：实现 `handle_update_issue`**

```rust
pub async fn handle_update_issue(...);
```

- [ ] **步骤 4：实现 `handle_query_issues`**

```rust
// 带分页和过滤的查询
pub async fn handle_query_issues(
    manager: &ConnectionManager,
    ws: &mut WebSocket,
    msg: ClientMessage,
) -> Result<(), WsError> {
    let payload: QueryIssuesCommand = msg.payload?;
    let issues = IssuesService::list_paginated(conn, &ctx, &payload.filters, payload.pagination)
        .map_err(WsError::App)?;

    ws.send_json(ServerMessage {
        type: "issues_list",
        payload: serde_json::to_value(&issues)?,
    }).await?;
    Ok(())
}
```

- [ ] **步骤 5：Commit**: `feat(ws): implement issue WebSocket handlers`

---

### Phase B：核心功能补全（P1）

#### 任务 B1：分页支持

**文件：** `momentum_core/src/db/repositories/issues.rs`

- [ ] **步骤 1：新增 `list_paginated` 方法**

```rust
pub fn list_paginated(
    conn: &mut PgConnection,
    ws_id: Uuid,
    filters: IssueFilters,
    cursor: Option<i32>,  // issue_number 或 created_at
    limit: i64,
) -> Result<Vec<Issue>, diesel::result::Error> {
    let mut q = issues::table
        .filter(issues::workspace_id.eq(ws_id))
        .order(issues::created_at.desc())
        .limit(limit);

    if let Some(cursor) = cursor {
        q = q.filter(issues::created_at.lt(cursor));
    }

    q.load(conn)
}
```

- [ ] **步骤 2：在 API 层暴露 cursor + limit 参数**

```rust
// issues.rs
#[derive(Deserialize)]
pub struct PaginationParams {
    pub limit: Option<i64>,  // default 50, max 100
    pub cursor: Option<i32>,
}
```

- [ ] **步骤 3：Commit**: `feat(repo): add cursor pagination to issue list`

---

#### 任务 B2：团队级 `issue_number`（ENG-123 格式）

**文件：** `momentum_core/src/db/repositories/issues.rs`

**现状**：DB 用 SERIAL 全局自增，不是团队隔离。

- [ ] **步骤 1：查看现有 migration**

```bash
grep -n "issue_number" momentum_core/migrations/2024-*/up.sql
```

- [ ] **步骤 2：创建新 migration**

```sql
-- 先删全局 SERIAL，改用触发器生成 team-scoped 序列

CREATE SEQUENCE IF NOT EXISTS seq_issues_team_{team_id}_start;
ALTER TABLE issues ALTER COLUMN issue_number SET DEFAULT nextval('seq_issues_team_{team_id}');
```

**实际做法**（触发器方案）：

```sql
-- 每个 team 独立的 issue_number 序列
CREATE SEQUENCE issues_team_seq START 1;

CREATE OR REPLACE FUNCTION generate_issue_number()
RETURNS TRIGGER AS $$
BEGIN
  NEW.issue_number := nextval('issues_team_seq');
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_issue_number
BEFORE INSERT ON issues
FOR EACH ROW
EXECUTE FUNCTION generate_issue_number();
```

**注意**：团队隔离需要按 team 分组，完整方案需要：
```sql
-- 每个 team 独立的序列
CREATE SEQUENCE issues_team_{team_key}_seq;

-- 触发器
NEW.issue_number := nextval('issues_team_' || NEW.team_key || '_seq');
```

- [ ] **步骤 3：Commit**: `feat(db): add team-scoped issue_number sequence`

---

#### 任务 B3：DB 层过滤（替代内存过滤）

**文件：** `momentum_core/src/db/repositories/issues.rs`

- [ ] **步骤 1：在 `list_by_workspace` 加 filter DSL**

```rust
pub fn list_by_workspace(
    conn: &mut PgConnection,
    ws_id: Uuid,
    filters: IssueFilters,
) -> Result<Vec<Issue>, diesel::result::Error> {
    let mut q = issues::table
        .filter(issues::team_id.eq(ws_id))
        .into_boxed();

    if let Some(team_id) = filters.team_id {
        q = q.filter(issues::team_id.eq(team_id));
    }
    if let Some(project_id) = filters.project_id {
        q = q.filter(issues::project_id.eq(project_id));
    }
    if let Some(assignee_id) = filters.assignee_id {
        q = q.filter(issues::assignee_id.eq(assignee_id));
    }
    if let Some(priority) = filters.priority {
        q = q.filter(issues::priority.eq(priority.as_str()));
    }
    if let Some(cycle_id) = filters.cycle_id {
        q = q.filter(issues::cycle_id.eq(cycle_id));
    }
    if let Some(search) = &filters.search {
        q = q.filter(issues::title.ilike(format!("%{}%", search)));
    }

    q.order(issues::created_at.desc())
        .load(conn)
}
```

- [ ] **步骤 2：Commit**: `feat(repo): move filtering to DB level`

---

### Phase C：高级功能（P2）

#### 任务 C1：Bulk update/delete

```rust
// Repository
pub fn bulk_update_status(conn: &mut PgConnection, ids: &[Uuid], status: &str) -> Result<usize> {
    diesel::update(issues::table.filter(issues::id.eq_any(ids)))
        .set(issues::status.eq(status))
        .execute(conn)
}
```

#### 任务 C2：sort 控制

```rust
// 新增 sort_by 参数到 list
pub enum IssueSortBy {
    CreatedAt,
    UpdatedAt,
    Priority,
    Status,
    Number,
}
```

#### 任务 C3：IssueFieldDefinitionRepo

```rust
pub struct IssueFieldDefinitionRepo;

impl IssueFieldDefinitionRepo {
    pub fn list_by_workspace(conn: &mut PgConnection, ws_id: Uuid) -> Result<Vec<IssueFieldDefinition>, diesel::result::Error> {
        issue_field_definitions::table
            .filter(issue_field_definitions::workspace_id.eq(ws_id))
            .order(issue_field_definitions::sort_order.asc())
            .load(conn)
    }
}
```

#### 任务 C4：Issue 关系（blocks/blocked_by/relates_to）

```sql
CREATE TABLE issue_relations (
  id UUID PRIMARY KEY,
  source_issue_id UUID REFERENCES issues(id),
  target_issue_id UUID REFERENCES issues(id),
  relation_type VARCHAR(20) CHECK (relation_type IN ('blocks', 'blocked_by', 'relates_to')),
  created_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

## 三、执行顺序

```
A1 (fix update_fields bug)   ←── 最紧急，先修
A2 (统一返回类型)
A3 (WS handlers)

B1 (分页)
B2 (team issue_number)
B3 (DB 层过滤)

C1 (bulk ops)
C2 (sort)
C3 (IssueFieldDefinitionRepo)
C4 (issue relations)
```

---

## 四、关键文件清单

| 文件 | 操作 |
|------|------|
| `momentum_core/src/db/repositories/issues.rs` | 修改 |
| `momentum_core/src/services/issues_service.rs` | 修改 |
| `momentum_api/src/routes/issues.rs` | 修改 |
| `momentum_api/src/websocket/commands/issues.rs` | 重写 |
| `momentum_api/src/websocket/commands/types.rs` | 可能新增 |
| `momentum_core/migrations/YYYYMMDD-0000XX_team_issue_numbers/` | 新增 |

---

## 五、自检标准

- [ ] `cargo test --package momentum_core` 全部通过
- [ ] `cargo check --workspace` 无错误
- [ ] `cargo clippy --workspace -- -D warnings` 无警告
- [ ] WebSocket `issue_created` 广播到 workspace 所有连接
- [ ] 创建/更新/列表 API 返回一致的数据结构
