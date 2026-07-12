# Issues WebSocket Commands 实现文档

## 概述

本文档描述了在 Momentum Backend 中实现的 Issues WebSocket 命令功能。该实现提供了完整的 Issue 管理功能，包括创建、更新、删除、查询和获取单个 Issue 的操作。

## 实现的功能

### 1. 创建 Issue (`create_issue`)
- **命令类型**: `create_issue`
- **功能**: 创建新的 Issue
- **必需字段**: `title`, `team_id`
- **可选字段**: `description`, `project_id`, `priority`, `assignee_id`, `workflow_id`, `workflow_state_id`, `label_ids`, `cycle_id`, `parent_issue_id`

### 2. 更新 Issue (`update_issue`)
- **命令类型**: `update_issue`
- **功能**: 更新现有 Issue
- **必需字段**: `issue_id`
- **可选字段**: 所有创建 Issue 的字段（除了 `parent_issue_id`）

### 3. 删除 Issue (`delete_issue`)
- **命令类型**: `delete_issue`
- **功能**: 删除指定的 Issue
- **必需字段**: `issue_id`

### 4. 查询 Issues (`query_issues`)
- **命令类型**: `query_issues`
- **功能**: 根据过滤条件查询 Issues
- **过滤条件**: `team_id`, `project_id`, `assignee_id`, `priority`, `search`

### 5. 获取单个 Issue (`get_issue`)
- **命令类型**: `get_issue`
- **功能**: 获取指定 Issue 的详细信息
- **必需字段**: `issue_id`

## 文件结构

### 新增文件
- `src/websocket/commands/issues.rs` - Issues 命令处理器实现

### 修改文件
- `src/websocket/commands/types.rs` - 添加 Issues 相关的命令类型和数据结构
- `src/websocket/commands/handler.rs` - 添加 Issues 命令的路由和处理逻辑
- `src/websocket/commands/mod.rs` - 导出新的 Issues 模块和类型

## 数据结构

### CreateIssueCommand
```rust
pub struct CreateIssueCommand {
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Uuid,
    pub priority: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
    pub cycle_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
}
```

### UpdateIssueCommand
```rust
pub struct UpdateIssueCommand {
    pub title: Option<String>,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub priority: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
}
```

### IssueFilters
```rust
pub struct IssueFilters {
    pub team_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub priority: Option<String>,
    pub search: Option<String>,
}
```

## 优先级支持

支持的优先级值：
- `"none"` - 无优先级
- `"low"` - 低优先级
- `"medium"` - 中等优先级
- `"high"` - 高优先级
- `"urgent"` - 紧急优先级

## 使用示例

### 创建 Issue
```json
{
  "type": "create_issue",
  "data": {
    "title": "修复登录页面bug",
    "description": "用户反馈登录页面在某些浏览器上无法正常显示",
    "team_id": "550e8400-e29b-41d4-a716-446655440000",
    "project_id": "550e8400-e29b-41d4-a716-446655440001",
    "priority": "high",
    "assignee_id": "550e8400-e29b-41d4-a716-446655440002",
    "label_ids": [
      "550e8400-e29b-41d4-a716-446655440003",
      "550e8400-e29b-41d4-a716-446655440004"
    ]
  },
  "request_id": "req_001"
}
```

### 查询 Issues
```json
{
  "type": "query_issues",
  "filters": {
    "team_id": "550e8400-e29b-41d4-a716-446655440000",
    "project_id": "550e8400-e29b-41d4-a716-446655440001",
    "assignee_id": "550e8400-e29b-41d4-a716-446655440002",
    "priority": "high",
    "search": "登录"
  },
  "request_id": "req_003"
}
```

## 错误处理

所有命令都包含完整的错误处理：
- 验证错误：字段验证失败
- 权限错误：用户无权访问工作区
- 业务错误：Issue 不存在、团队不存在等
- 系统错误：数据库连接失败等

## 演示程序

运行演示程序查看完整的命令示例：
```bash
cargo run --example issues_websocket_demo
```

## 集成说明

该实现完全集成了现有的：
- IssuesService - 业务逻辑处理
- 数据库模型和仓库
- 权限验证和上下文管理
- WebSocket 安全机制
- 幂等性控制

所有功能都遵循现有的代码模式和架构设计。
