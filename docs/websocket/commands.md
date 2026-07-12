# WebSocket 命令目录

> 对应代码：`momentum_api/src/websocket/commands/types.rs`（约 65 个命令变体）
> 请求/响应格式：见 `README.md` §2

---

## 通用命令（连接管理）

| 命令 | 用途 |
|---|---|
| `Subscribe` | 订阅主题（`issues` / `workspace:{wid}` / `project:{pid}` 等） |
| `Unsubscribe` | 取消订阅 |
| `GetConnectionInfo` | 查询当前连接元数据 |
| `Ping` | 心跳 |
| `GetFeatureFlags` | 查询功能开关 |

### Subscribe 示例

```json
{
  "command": "Subscribe",
  "data": {
    "topics": ["issues", "workspace:abc-123"]
  },
  "request_id": "sub-1"
}
```

---

## 标签（Labels）

| 命令 | 用途 |
|---|---|
| `CreateLabel` | 创建 |
| `UpdateLabel` | 更新 |
| `DeleteLabel` | 删除 |
| `GetLabel` | 按 ID 获取 |
| `QueryLabels` | 列表查询 |
| `BatchCreateLabels` | 批量创建 |
| `BatchUpdateLabels` | 批量更新 |
| `BatchDeleteLabels` | 批量删除 |

---

## 团队（Teams）

| 命令 | 用途 |
|---|---|
| `CreateTeam` / `UpdateTeam` / `DeleteTeam` / `GetTeam` / `QueryTeams` | CRUD |
| `GetTeamWorkflowStatuses` / `CreateTeamWorkflowStatus` / `UpdateTeamWorkflowStatus` / `DeleteTeamWorkflowStatus` | 工作流状态管理 |
| `AddTeamMember` / `UpdateTeamMember` / `RemoveTeamMember` / `ListTeamMembers` | 成员管理 |

---

## 工作区（Workspaces）

| 命令 | 用途 |
|---|---|
| `CreateWorkspace` / `UpdateWorkspace` / `DeleteWorkspace` | CRUD |
| `GetWorkspace` / `GetCurrentWorkspace` | 查询 |
| `SwitchWorkspace` | 切换当前工作区 |
| `QueryWorkspaceMembers` | 成员列表 |
| `GetWorkspaceMember` / `UpdateWorkspaceMember` / `DeleteWorkspaceMember` | 单成员操作 |
| `InviteWorkspaceMember` | 邀请 |
| `AcceptInvitation` / `GetInvitation` | 邀请接受/查询 |

### Workspace 命令详情

> 详细字段说明见 `docs/websocket/workspace-commands.md`（从根目录迁入的历史权威参考）

通用 Data 字段模板：

```json
{
  "command": "CreateWorkspace",
  "data": {
    "name": "My Workspace",
    "url_key": "my-workspace",   // 最长 10 字符
    "logo_url": "https://..."
  },
  "request_id": "req-001"
}
```

错误码：

| error_type | code | 含义 |
|---|---|---|
| `conflict` | `WORKSPACE_URL_KEY_EXISTS` | URL key 重复 |
| `conflict` | `PENDING_INVITATION` | 已有待处理邀请 |
| `conflict` | `INVITATION_ALREADY_ACCEPTED` | 邀请已被接受 |
| `forbidden` | `FORBIDDEN` | 无权限 |
| `not_found` | `NOT_FOUND` | 资源不存在 |

---

## 项目状态（Project Statuses）

| 命令 | 用途 |
|---|---|
| `CreateProjectStatus` / `UpdateProjectStatus` / `DeleteProjectStatus` | CRUD |
| `QueryProjectStatuses` / `GetProjectStatusById` | 查询 |

---

## 项目（Projects）

| 命令 | 用途 |
|---|---|
| `CreateProject` / `UpdateProject` / `DeleteProject` / `GetProject` / `QueryProjects` | CRUD |

---

## 任务（Issues）

| 命令 | 用途 |
|---|---|
| `CreateIssue` / `UpdateIssue` / `DeleteIssue` / `GetIssue` / `QueryIssues` | CRUD |
| `QueryIssuePriorities` | 优先级枚举查询 |

---

## 周期（Cycles）

| 命令 | 用途 |
|---|---|
| `CreateCycle` / `UpdateCycle` / `DeleteCycle` / `GetCycle` / `QueryCycles` | CRUD |

---

## 评论（Comments）

| 命令 | 用途 |
|---|---|
| `CreateComment` / `UpdateComment` / `DeleteComment` / `QueryComments` | CRUD |

---

## 用户资料（Profile）

| 命令 | 用途 |
|---|---|
| `UpdateProfile` / `QueryProfile` | 更新/查询 |

---

## 事件订阅语义

| 主题 | 触发场景 | 负载示例 |
|---|---|---|
| `issues` | 当前工作区内所有 issue 事件 | `{event: "issue.created", data: {...}}` |
| `workspace:{wid}` | 工作区元数据变更 | `{event: "workspace.updated", ...}` |
| `project:{pid}` | 单项目事件 | `{event: "project.deleted", ...}` |

> ⚠️ 当前 Registry 实现存在"双分发"问题——Legacy 与 Registry 两套实现并存，详见 [registry-vs-legacy.md](./registry-vs-legacy.md)。

---

**最后更新**：2026-07-12