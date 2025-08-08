# 统一API返回结构设计

## 设计原则

- **一致性**: 所有API端点使用相同的响应结构
- **可预测性**: 前端可以用统一的方式处理所有响应
- **扩展性**: 支持未来功能扩展
- **向后兼容**: 尽量保持现有API的兼容性

## 标准响应结构

### 基础响应结构

```typescript
interface ApiResponse<T = any> {
  success: boolean;           // 操作是否成功
  code: number;              // HTTP状态码或业务状态码
  message: string;           // 用户友好的消息
  data?: T;                  // 实际数据（成功时）
  meta?: ResponseMeta;       // 元数据信息
  errors?: ErrorDetail[];    // 详细错误信息（失败时）
  timestamp: string;         // 响应时间戳
}

interface ResponseMeta {
  request_id?: string;       // 请求追踪ID
  pagination?: Pagination;   // 分页信息
  total_count?: number;      // 总记录数
  execution_time_ms?: number; // 执行时间（毫秒）
}

interface Pagination {
  page: number;              // 当前页码
  per_page: number;          // 每页数量
  total_pages: number;       // 总页数
  has_next: boolean;         // 是否有下一页
  has_prev: boolean;         // 是否有上一页
}

interface ErrorDetail {
  field?: string;            // 错误字段（表单验证错误）
  code: string;              // 错误代码
  message: string;           // 错误描述
}
```

## 具体场景示例

### 1. 成功响应示例

#### 单个资源 (GET /auth/profile)
```json
{
  "success": true,
  "code": 200,
  "message": "Profile retrieved successfully",
  "data": {
    "id": "49a65ecd-f0b7-40f4-874b-8d625214cb02",
    "email": "user@example.com",
    "username": "username",
    "name": "Full Name",
    "current_workspace_id": "workspace-uuid",
    "workspaces": [...],
    "teams": [...]
  },
  "meta": {
    "request_id": "req_123456789",
    "execution_time_ms": 45
  },
  "timestamp": "2025-07-26T08:20:14Z"
}
```

#### 资源列表 (GET /users)
```json
{
  "success": true,
  "code": 200,
  "message": "Users retrieved successfully",
  "data": [
    {
      "id": "user-uuid-1",
      "name": "User 1",
      "email": "user1@example.com"
    },
    {
      "id": "user-uuid-2",
      "name": "User 2",
      "email": "user2@example.com"
    }
  ],
  "meta": {
    "request_id": "req_123456790",
    "total_count": 25,
    "pagination": {
      "page": 1,
      "per_page": 10,
      "total_pages": 3,
      "has_next": true,
      "has_prev": false
    }
  },
  "timestamp": "2025-07-26T08:20:14Z"
}
```

#### 创建资源 (POST /auth/register)
```json
{
  "success": true,
  "code": 201,
  "message": "User registered successfully",
  "data": {
    "access_token": "eyJ0eXAiOiJKV1Q...",
    "refresh_token": "eyJ0eXAiOiJKV1Q...",
    "token_type": "Bearer",
    "expires_in": 3600,
    "user": {
      "id": "user-uuid",
      "email": "user@example.com",
      "username": "username",
      "name": "Full Name"
    }
  },
  "meta": {
    "request_id": "req_123456791"
  },
  "timestamp": "2025-07-26T08:20:14Z"
}
```

#### 操作确认 (POST /auth/switch-workspace)
```json
{
  "success": true,
  "code": 200,
  "message": "Workspace switched successfully",
  "data": {
    "current_workspace_id": "new-workspace-uuid",
    "workspace_name": "New Workspace"
  },
  "meta": {
    "request_id": "req_123456792"
  },
  "timestamp": "2025-07-26T08:20:14Z"
}
```

### 2. 错误响应示例

#### 验证错误 (400 Bad Request)
```json
{
  "success": false,
  "code": 400,
  "message": "Validation failed",
  "errors": [
    {
      "field": "email",
      "code": "INVALID_FORMAT",
      "message": "Email format is invalid"
    },
    {
      "field": "password",
      "code": "TOO_SHORT",
      "message": "Password must be at least 8 characters"
    }
  ],
  "meta": {
    "request_id": "req_123456793"
  },
  "timestamp": "2025-07-26T08:20:14Z"
}
```

#### 认证错误 (401 Unauthorized)
```json
{
  "success": false,
  "code": 401,
  "message": "Authentication required",
  "errors": [
    {
      "code": "INVALID_TOKEN",
      "message": "Access token is invalid or expired"
    }
  ],
  "meta": {
    "request_id": "req_123456794"
  },
  "timestamp": "2025-07-26T08:20:14Z"
}
```

#### 权限错误 (403 Forbidden)
```json
{
  "success": false,
  "code": 403,
  "message": "Access denied",
  "errors": [
    {
      "code": "INSUFFICIENT_PERMISSIONS",
      "message": "You don't have access to this workspace"
    }
  ],
  "meta": {
    "request_id": "req_123456795"
  },
  "timestamp": "2025-07-26T08:20:14Z"
}
```

#### 资源未找到 (404 Not Found)
```json
{
  "success": false,
  "code": 404,
  "message": "Resource not found",
  "errors": [
    {
      "code": "USER_NOT_FOUND",
      "message": "User with ID 'invalid-uuid' does not exist"
    }
  ],
  "meta": {
    "request_id": "req_123456796"
  },
  "timestamp": "2025-07-26T08:20:14Z"
}
```

#### 服务器错误 (500 Internal Server Error)
```json
{
  "success": false,
  "code": 500,
  "message": "Internal server error",
  "errors": [
    {
      "code": "DATABASE_CONNECTION_FAILED",
      "message": "Failed to connect to database"
    }
  ],
  "meta": {
    "request_id": "req_123456797"
  },
  "timestamp": "2025-07-26T08:20:14Z"
}
```

## 状态码规范

### HTTP状态码映射
- **200 OK**: 成功获取资源
- **201 Created**: 成功创建资源
- **204 No Content**: 成功删除资源（无返回内容）
- **400 Bad Request**: 请求参数错误
- **401 Unauthorized**: 认证失败
- **403 Forbidden**: 权限不足
- **404 Not Found**: 资源未找到
- **409 Conflict**: 资源冲突（如邮箱已存在）
- **422 Unprocessable Entity**: 业务逻辑错误
- **429 Too Many Requests**: 请求频率超限
- **500 Internal Server Error**: 服务器内部错误

### 业务错误码设计
```
AUTH_001: 无效的邮箱格式
AUTH_002: 密码强度不足
AUTH_003: 用户不存在
AUTH_004: 密码错误
AUTH_005: 账号已被禁用

USER_001: 用户名已存在
USER_002: 邮箱已存在
USER_003: 用户资料不完整

WORKSPACE_001: 工作空间不存在
WORKSPACE_002: 无访问权限
WORKSPACE_003: 工作空间名称重复

TEAM_001: 团队不存在
TEAM_002: 非团队成员
TEAM_003: 权限不足

SYSTEM_001: 数据库连接失败
SYSTEM_002: 缓存服务不可用
SYSTEM_003: 外部服务调用失败
```

## 实现建议

### 1. Rust实现结构
```rust
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ErrorDetail>>,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct ResponseMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub code: String,
    pub message: String,
}
```

### 2. 便捷构造函数
```rust
impl<T> ApiResponse<T> {
    pub fn success(data: T, message: &str) -> Self {
        Self {
            success: true,
            code: 200,
            message: message.to_string(),
            data: Some(data),
            meta: None,
            errors: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn error(code: u16, message: &str, errors: Vec<ErrorDetail>) -> Self {
        Self {
            success: false,
            code,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: Some(errors),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}
```

## 迁移策略

### 阶段1: 新API使用统一格式
- 所有新开发的API端点使用新格式
- 现有API保持不变，避免破坏性更改

### 阶段2: 版本化迁移
- 通过API版本号区分 (v1: 老格式, v2: 新格式)
- 例如: `/api/v1/users` vs `/api/v2/users`

### 阶段3: 逐步迁移现有API
- 提供迁移工具和文档
- 给前端团队足够的迁移时间
- 最终废弃老版本API

## 前端集成优势

### 1. 统一的错误处理
```typescript
async function apiCall<T>(url: string): Promise<T> {
  const response = await fetch(url);
  const apiResponse: ApiResponse<T> = await response.json();

  if (!apiResponse.success) {
    throw new ApiError(apiResponse.message, apiResponse.errors);
  }

  return apiResponse.data!;
}
```

### 2. 自动化类型生成
- 可以根据Rust结构体自动生成TypeScript类型
- 保证前后端类型一致性

### 3. 统一的Loading和Error状态
- 前端框架可以基于统一格式构建通用组件
- 自动处理loading、success、error状态

你觉得这个设计如何？有什么需要调整或补充的地方吗？