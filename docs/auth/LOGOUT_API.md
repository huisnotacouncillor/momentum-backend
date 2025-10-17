# 登出 API 文档

## 概述

用户登出功能允许已认证的用户安全地退出系统，使其所有活动会话失效。

## API 接口

### 用户登出

**端点：** `POST /auth/logout`

**认证：** 需要（Bearer Token）

**请求头：**
```
Authorization: Bearer <access_token>
```

**请求体：** 无

**响应示例：**

成功响应（200 OK）：
```json
{
  "success": true,
  "message": "Logout successful",
  "data": null,
  "code": null,
  "field": null
}
```

错误响应（401 Unauthorized）：
```json
{
  "success": false,
  "message": "Unauthorized",
  "data": null,
  "code": "UNAUTHORIZED",
  "field": null
}
```

## 功能说明

### 登出行为

1. **会话失效**：登出操作会将用户的所有活动会话标记为失效（`is_active = false`）
2. **缓存清理**：自动清除 Redis 中的所有用户相关缓存，包括：
   - `user:{user_id}` - 用户基本信息
   - `user_profile:{user_id}` - 用户详细资料
   - `user_workspace:{user_id}` - 用户当前工作空间
3. **Token 处理**：客户端应该在收到成功响应后立即删除本地存储的 access_token 和 refresh_token
4. **重定向**：登出后，客户端应将用户重定向到登录页面

### 安全考虑

- 登出接口需要有效的认证令牌
- 即使会话已在数据库中失效，客户端也必须删除本地令牌
- 建议在敏感操作后提示用户登出
- 支持多设备登出（所有设备的会话都会失效）
- Redis 缓存清理失败不会影响登出流程（会话仍然失效，缓存会自然过期）

## 使用示例

### cURL 示例

```bash
curl -X POST http://localhost:8000/auth/logout \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -H "Content-Type: application/json"
```

### JavaScript (Fetch API) 示例

```javascript
async function logout() {
  const token = localStorage.getItem('access_token');

  try {
    const response = await fetch('http://localhost:8000/auth/logout', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      }
    });

    const data = await response.json();

    if (data.success) {
      // 清除本地存储的令牌
      localStorage.removeItem('access_token');
      localStorage.removeItem('refresh_token');

      // 重定向到登录页面
      window.location.href = '/login';
    } else {
      console.error('Logout failed:', data.message);
    }
  } catch (error) {
    console.error('Logout error:', error);
  }
}
```

### Rust 示例

```rust
use reqwest;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = "your-access-token-here";
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8000/auth/logout")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let result: Value = response.json().await?;
    println!("Logout response: {}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
```

## 完整工作流程

### 1. 用户登录
```
POST /auth/login
-> 获得 access_token 和 refresh_token
-> 存储令牌到本地
```

### 2. 使用受保护的资源
```
GET /auth/profile
Authorization: Bearer <access_token>
-> 访问用户数据
```

### 3. 用户登出
```
POST /auth/logout
Authorization: Bearer <access_token>
-> 会话失效
-> 删除本地令牌
-> 重定向到登录页
```

## 最佳实践

1. **自动登出**
   - 在 Token 过期时自动触发登出流程
   - 长时间不活跃后提示用户重新登录

2. **全局登出**
   - 提供"登出所有设备"选项（当前实现已默认支持）
   - 在密码修改后强制登出所有设备

3. **错误处理**
   - 即使登出 API 调用失败，也要清除本地令牌
   - 为用户提供清晰的错误提示

4. **UI/UX**
   - 登出按钮应该在所有页面都可访问
   - 登出过程中显示加载状态
   - 登出成功后显示确认消息

## 相关接口

- [用户注册](AUTH_README.md#1-用户注册)
- [用户登录](AUTH_README.md#2-用户登录)
- [刷新Token](AUTH_README.md#3-刷新token)
- [获取用户信息](AUTH_README.md#4-获取用户信息)

## 技术实现

### 数据库操作

登出时会执行以下 SQL 操作：

```sql
UPDATE user_sessions
SET is_active = false
WHERE user_id = $1 AND is_active = true;
```

这确保了用户的所有活动会话都被正确失效。

### Redis 缓存清理

登出操作会自动清除 Redis 中的用户相关缓存：

```rust
// 清除的缓存键
let cache_keys = vec![
    format!("user:{}", user_id),           // 用户基本信息
    format!("user_profile:{}", user_id),   // 用户详细资料
    format!("user_workspace:{}", user_id), // 用户工作空间
];

// 批量删除
for key in cache_keys {
    redis_conn.del(&key).await;
}
```

**容错机制**：
- 如果 Redis 连接失败，登出操作仍然成功（数据库会话已失效）
- 即使缓存清理失败，缓存也会在 TTL 到期后自动过期
- 所有 Redis 操作失败都会记录日志但不会阻断登出流程

### 后续改进

未来可能的增强功能：

1. **选择性登出**：允许用户选择登出特定设备
2. **登出回调**：支持 WebSocket 通知其他设备登出
3. **审计日志**：记录登出时间和设备信息
4. **会话管理界面**：允许用户查看和管理所有活动会话

