# Issue Workflow Transitions API

## 概述

本文档描述了通过issue ID获取可用workflow transitions的API接口实现。

## API接口

### GET /issues/{issue_id}/transitions

获取指定issue可用的workflow状态转换。

#### 请求参数

- **Path参数**:
  - `issue_id` (UUID): 要查询的issue ID

- **Headers**:
  - `Authorization: Bearer <token>`: 认证token

#### 响应格式

```json
{
  "success": true,
  "message": "Available transitions retrieved successfully",
  "data": [
    {
      "id": "transition-uuid",
      "workflow_id": "workflow-uuid",
      "from_state_id": "from-state-uuid", // 可为null
      "to_state_id": "to-state-uuid",
      "name": "Move to Done",
      "description": "Move issue to completed state",
      "created_at": "2025-01-01T00:00:00Z",
      "from_state": {
        "id": "from-state-uuid",
        "workflow_id": "workflow-uuid",
        "name": "In Progress",
        "description": "Issues currently being worked on",
        "color": "#F1BF00",
        "category": "started",
        "position": 1,
        "is_default": false,
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z"
      }, // 可为null
      "to_state": {
        "id": "to-state-uuid",
        "workflow_id": "workflow-uuid",
        "name": "Done",
        "description": "Completed issues",
        "color": "#0000FF",
        "category": "completed",
        "position": 1,
        "is_default": false,
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z"
      }
    }
  ]
}
```

#### 错误响应

**400 Bad Request**:
```json
{
  "error": "No current workspace selected"
}
```

```json
{
  "error": "Issue does not have an associated workflow"
}
```

**404 Not Found**:
```json
{
  "error": "Issue not found or access denied"
}
```

**500 Internal Server Error**:
```json
{
  "error": "Database connection failed"
}
```

```json
{
  "error": "Failed to get issue"
}
```

```json
{
  "error": "Failed to load workflow states"
}
```

```json
{
  "error": "Failed to load workflow transitions"
}
```

## 实现细节

### 数据模型

#### IssueTransitionResponse

```rust
#[derive(Serialize, Clone)]
pub struct IssueTransitionResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub from_state_id: Option<Uuid>,
    pub to_state_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub from_state: Option<WorkflowStateResponse>,
    pub to_state: WorkflowStateResponse,
}
```

### 业务逻辑

1. **权限验证**: 验证用户对issue的访问权限
2. **Workflow检查**: 确保issue有关联的workflow
3. **状态查询**: 获取issue的当前workflow state
4. **转换查询**: 查找可用的transitions：
   - `from_state_id` 为 `NULL` 的转换（可从任何状态转换）
   - `from_state_id` 匹配当前状态的转换
5. **数据组装**: 将转换和状态信息组合成响应格式

### 数据库查询

```sql
-- 获取issue及其权限验证
SELECT i.* FROM issues i
INNER JOIN teams t ON i.team_id = t.id
WHERE i.id = ? AND t.workspace_id = ?

-- 获取workflow states
SELECT * FROM workflow_states
WHERE workflow_id = ?

-- 获取可用transitions
SELECT * FROM workflow_transitions
WHERE workflow_id = ?
AND (from_state_id IS NULL OR from_state_id = ?)
```

## 使用场景

1. **前端状态转换UI**: 动态显示可用的状态转换按钮
2. **工作流引擎**: 确定issue可执行的操作
3. **状态机验证**: 验证状态转换的有效性
4. **用户界面**: 动态生成状态选择器

## 注意事项

- 需要有效的认证token
- issue必须属于用户当前工作空间
- issue必须有关联的workflow
- 返回的transitions包含完整的状态信息
- 支持从任何状态转换（from_state_id为null）
- 支持从特定状态转换（from_state_id匹配当前状态）

## 示例请求

```bash
curl -H "Authorization: Bearer <token>" \
     "http://localhost:3000/issues/123e4567-e89b-12d3-a456-426614174000/transitions"
```

## 相关文件

- `src/db/models/workflow.rs`: 数据模型定义
- `src/routes/workflows.rs`: API处理函数
- `src/routes/mod.rs`: 路由配置
- `examples/issue_transitions_demo.rs`: 使用示例
