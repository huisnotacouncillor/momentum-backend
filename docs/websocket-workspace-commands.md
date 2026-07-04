# Workspace WebSocket 命令文档

## 连接方式

```javascript
WebSocket://{host}/ws?token={jwt_token}
```

认证通过 URL query parameter 传递 JWT token。

---

## 通用请求格式

```json
{
  "command": "CommandName",
  "data": { /* 命令数据 */ },
  "request_id": "可选，用于追踪请求" }
}
```

## 通用响应格式

成功：
```json
{
  "success": true,
  "request_id": "请求中的request_id",
  "data": { /* 响应数据 */ }
}
```

错误：
```json
{
  "success": false,
  "request_id": "请求中的request_id",
  "error": {
    "code": "ERROR_CODE",
    "message": "错误描述",
    "field": null,
    "details": null,
    "error_type": "validation|not_found|conflict|forbidden|internal"
  }
}
```

---

## Workspace 命令

### 1. CreateWorkspace - 创建 Workspace

**Command:**
```json
{
  "command": "CreateWorkspace",
  "data": {
    "name": "My Workspace",
    "url_key": "my-workspace",
    "logo_url": "https://example.com/logo.png"
  },
  "request_id": "req-001"
}
```

**Data 字段：**
| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | String | ✅ | Workspace 名称 |
| `url_key` | String | ✅ | 唯一标识符，最多10字符 |
| `logo_url` | String\|null | ❌ | Logo URL |

**预期响应：**
```json
{
  "success": true,
  "request_id": "req-001",
  "data": {
    "id": "uuid",
    "name": "My Workspace",
    "url_key": "my-workspace",
    "logo_url": null,
    "created_at": "2026-07-04T10:00:00",
    "updated_at": "2026-07-04T10:00:00"
  }
}
```

---

### 2. UpdateWorkspace - 更新 Workspace

**Command:**
```json
{
  "command": "UpdateWorkspace",
  "data": {
    "workspace_id": "uuid",
    "data": {
      "name": "Updated Name",
      "url_key": "updated-key",
      "logo_url": "https://example.com/new-logo.png"
    }
  },
  "request_id": "req-002"
}
```

**Data 字段：**
| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `workspace_id` | Uuid | ✅ | Workspace ID |
| `data.name` | String\|null | ❌ | 新的名称 |
| `data.url_key` | String\|null | ❌ | 新的唯一标识符 |
| `data.logo_url` | String\|null | ❌ | 新的 Logo URL |

**预期响应：**
```json
{
  "success": true,
  "request_id": "req-002",
  "data": {
    "id": "uuid",
    "name": "Updated Name",
    "url_key": "updated-key",
    "logo_url": "https://example.com/new-logo.png",
    "created_at": "2026-07-04T10:00:00",
    "updated_at": "2026-07-04T11:00:00"
  }
}
```

---

### 3. DeleteWorkspace - 删除 Workspace

**Command:**
```json
{
  "command": "DeleteWorkspace",
  "data": {
    "workspace_id": "uuid"
  },
  "request_id": "req-003"
}
```

**Data 字段：**
| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `workspace_id` | Uuid | ✅ | 要删除的 Workspace ID |

**预期响应：**
```json
{
  "success": true,
  "request_id": "req-003",
  "data": null
}
```

---

### 4. GetCurrentWorkspace - 获取当前 Workspace

**Command:**
```json
{
  "command": "GetCurrentWorkspace",
  "data": {},
  "request_id": "req-004"
}
```

**预期响应：**
```json
{
  "success": true,
  "request_id": "req-004",
  "data": {
    "id": "uuid",
    "name": "My Workspace",
    "url_key": "my-workspace",
    "logo_url": null,
    "created_at": "2026-07-04T10:00:00",
    "updated_at": "2026-07-04T10:00:00"
  }
}
```

---

## Workspace 成员命令

### 5. InviteWorkspaceMember - 邀请成员

**Command:**
```json
{
  "command": "InviteWorkspaceMember",
  "data": {
    "email": "user@example.com",
    "role": "member"
  },
  "request_id": "req-005"
}
```

**Data 字段：**
| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `email` | String | ✅ | 被邀请者邮箱 |
| `role` | String | ✅ | 角色：`owner`、`admin`、`member` |

**role 可选值：**
- `owner` - 所有者
- `admin` - 管理员
- `member` - 成员

**预期响应：**
```json
{
  "success": true,
  "request_id": "req-005",
  "data": {
    "id": "uuid",
    "email": "user@example.com",
    "role": "member",
    "status": "pending",
    "invited_by": "uuid",
    "inviter_name": "inviter",
    "inviter_avatar_url": null,
    "workspace_id": "uuid",
    "workspace_name": "My Workspace",
    "workspace_logo_url": null,
    "expires_at": "2026-07-11T10:00:00",
    "created_at": "2026-07-04T10:00:00",
    "updated_at": "2026-07-04T10:00:00"
  }
}
```

---

### 6. AcceptInvitation - 接受邀请

**Command:**
```json
{
  "command": "AcceptInvitation",
  "data": {
    "invitation_id": "uuid"
  },
  "request_id": "req-006"
}
```

**Data 字段：**
| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `invitation_id` | Uuid | ✅ | 邀请 ID |

**预期响应：**
```json
{
  "success": true,
  "request_id": "req-006",
  "data": {
    "id": "uuid",
    "email": "user@example.com",
    "role": "member",
    "status": "accepted",
    "invited_by": "uuid",
    "inviter_name": "inviter",
    "inviter_avatar_url": null,
    "workspace_id": "uuid",
    "workspace_name": "My Workspace",
    "workspace_logo_url": null,
    "expires_at": "2026-07-11T10:00:00",
    "created_at": "2026-07-04T10:00:00",
    "updated_at": "2026-07-04T10:30:00"
  }
}
```

---

### 7. QueryWorkspaceMembers - 查询成员列表

**Command:**
```json
{
  "command": "QueryWorkspaceMembers",
  "data": {
    "filters": {
      "role": "member",
      "user_id": "uuid",
      "search": "john"
    }
  },
  "request_id": "req-007"
}
```

**Data 字段：**
| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `filters.role` | String\|null | ❌ | 按角色过滤：`owner`、`admin`、`member` |
| `filters.user_id` | Uuid\|null | ❌ | 按用户 ID 过滤 |
| `filters.search` | String\|null | ❌ | 搜索用户名/邮箱 |

**预期响应：**
```json
{
  "success": true,
  "request_id": "req-007",
  "data": [
    {
      "id": "uuid",
      "user_id": "uuid",
      "workspace_id": "uuid",
      "role": "member",
      "user": {
        "id": "uuid",
        "name": "John Doe",
        "username": "john",
        "email": "john@example.com",
        "avatar_url": null
      },
      "created_at": "2026-07-04T10:00:00",
      "updated_at": "2026-07-04T10:00:00"
    }
  ]
}
```

---

## 错误码

| error_type | code | 说明 |
|------------|------|------|
| `not_found` | `NOT_FOUND` | Workspace 或 Invitation 不存在 |
| `conflict` | `WORKSPACE_URL_KEY_EXISTS` | URL key 已存在 |
| `conflict` | `PENDING_INVITATION` | 已有待处理邀请 |
| `conflict` | `INVITATION_ALREADY_ACCEPTED` | 邀请已被接受 |
| `forbidden` | `FORBIDDEN` | 无权限操作 |
| `validation` | `VALIDATION_ERROR` | 参数验证失败 |
| `internal` | `INTERNAL_ERROR` | 服务器内部错误 |

---

## 状态说明

### InvitationStatus
| 状态 | 说明 |
|------|------|
| `pending` | 待处理 |
| `accepted` | 已接受 |
| `declined` | 已拒绝 |
| `cancelled` | 已撤销 |

### WorkspaceMemberRole
| 角色 | 说明 |
|------|------|
| `owner` | 所有者 |
| `admin` | 管理员 |
| `member` | 成员 |

---

## 注意事项

1. **当前状态**：Workspace WS 命令处理器尚未实现，调用会返回 `internal error`
2. **request_id**：用于追踪请求，非必填但建议提供
3. **权限**：部分操作需要 Owner 或 Admin 权限
