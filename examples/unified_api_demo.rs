use serde_json::json;
use uuid::Uuid;

/// Demo showing the unified API response structure
fn main() {
    println!("🎯 统一API返回结构演示");
    println!("====================\n");

    // 成功响应示例
    println!("✅ 成功响应示例");
    println!("--------------");

    // 1. 用户注册成功
    let register_success = json!({
        "success": true,
        "code": 201,
        "message": "User registered successfully",
        "data": {
            "access_token": "eyJ0eXAiOiJKV1Q...",
            "refresh_token": "eyJ0eXAiOiJKV1Q...",
            "token_type": "Bearer",
            "expires_in": 3600,
            "user": {
                "id": Uuid::new_v4(),
                "email": "user@example.com",
                "username": "newuser",
                "name": "New User"
            }
        },
        "meta": {
            "request_id": "req_12345",
            "execution_time_ms": 156
        },
        "timestamp": "2025-07-26T08:20:14Z"
    });

    println!("📄 POST /auth/register");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&register_success).unwrap()
    );

    // 2. 获取用户列表（带分页）
    let users_list = json!({
        "success": true,
        "code": 200,
        "message": "Users retrieved successfully",
        "data": [
            {
                "id": Uuid::new_v4(),
                "name": "User 1",
                "email": "user1@example.com",
                "username": "user1"
            },
            {
                "id": Uuid::new_v4(),
                "name": "User 2",
                "email": "user2@example.com",
                "username": "user2"
            }
        ],
        "meta": {
            "request_id": "req_12346",
            "total_count": 25,
            "pagination": {
                "page": 1,
                "per_page": 10,
                "total_pages": 3,
                "has_next": true,
                "has_prev": false
            },
            "execution_time_ms": 89
        },
        "timestamp": "2025-07-26T08:20:15Z"
    });

    println!("📄 GET /users");
    println!("{}\n", serde_json::to_string_pretty(&users_list).unwrap());

    // 3. Workspace切换成功
    let workspace_switch = json!({
        "success": true,
        "code": 200,
        "message": "Workspace switched successfully",
        "data": {
            "current_workspace_id": Uuid::new_v4(),
            "workspace_name": "New Workspace"
        },
        "meta": {
            "request_id": "req_12347"
        },
        "timestamp": "2025-07-26T08:20:16Z"
    });

    println!("📄 POST /auth/switch-workspace");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&workspace_switch).unwrap()
    );

    // 错误响应示例
    println!("❌ 错误响应示例");
    println!("--------------");

    // 1. 验证错误
    let validation_error = json!({
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
            "request_id": "req_12348"
        },
        "timestamp": "2025-07-26T08:20:17Z"
    });

    println!("📄 POST /auth/register (验证失败)");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&validation_error).unwrap()
    );

    // 2. 认证错误
    let auth_error = json!({
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
            "request_id": "req_12349"
        },
        "timestamp": "2025-07-26T08:20:18Z"
    });

    println!("📄 GET /auth/profile (认证失败)");
    println!("{}\n", serde_json::to_string_pretty(&auth_error).unwrap());

    // 3. 权限错误
    let permission_error = json!({
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
            "request_id": "req_12350"
        },
        "timestamp": "2025-07-26T08:20:19Z"
    });

    println!("📄 POST /auth/switch-workspace (权限不足)");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&permission_error).unwrap()
    );

    // 4. 业务逻辑错误
    let business_error = json!({
        "success": false,
        "code": 409,
        "message": "Resource conflict",
        "errors": [
            {
                "field": "email",
                "code": "USER_002",
                "message": "Email address already exists"
            }
        ],
        "meta": {
            "request_id": "req_12351"
        },
        "timestamp": "2025-07-26T08:20:20Z"
    });

    println!("📄 POST /auth/register (邮箱已存在)");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&business_error).unwrap()
    );

    // 前端集成示例
    println!("🔧 前端集成优势");
    println!("==============");

    println!(
        "
🎯 统一的TypeScript类型：
```typescript
interface ApiResponse<T = any> {{
  success: boolean;
  code: number;
  message: string;
  data?: T;
  meta?: ResponseMeta;
  errors?: ErrorDetail[];
  timestamp: string;
}}
```

🔄 统一的错误处理：
```typescript
async function apiCall<T>(url: string): Promise<T> {{
  const response = await fetch(url);
  const apiResponse: ApiResponse<T> = await response.json();

  if (!apiResponse.success) {{
    throw new ApiError(apiResponse.message, apiResponse.errors);
  }}

  return apiResponse.data!;
}}
```

📊 自动化状态管理：
```typescript
const {{ data, loading, error }} = useApi('/users');
// 基于统一格式，可以构建通用的React hooks
```
"
    );

    println!("\n📋 主要优势：");
    println!("├── 🎯 前端可预测的响应格式");
    println!("├── 🔄 统一的错误处理机制");
    println!("├── 📊 支持分页和元数据");
    println!("├── 🏷️  详细的业务错误码");
    println!("├── 📈 请求追踪和性能监控");
    println!("├── 🔒 类型安全的前后端通信");
    println!("└── 🚀 更好的开发者体验");

    println!("\n🛠️  实施建议：");
    println!("1. 新API立即采用统一格式");
    println!("2. 现有API逐步迁移（v1 -> v2）");
    println!("3. 前端构建通用的API客户端");
    println!("4. 建立错误码文档和规范");
    println!("5. 自动化测试验证响应格式");
}
