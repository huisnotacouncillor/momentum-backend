# JWT认证系统设计文档

## 功能概述

本系统实现了一个完整的JWT token认证功能，支持用户注册、登录、token刷新和登出，并为未来的OAuth集成预留了扩展空间。

## 数据库设计

### 1. 用户表 (users)
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(100) UNIQUE NOT NULL,
    name TEXT NOT NULL,
    avatar_url TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### 2. 用户认证表 (user_credentials)
```sql
CREATE TABLE user_credentials (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_type VARCHAR(50) NOT NULL, -- 'password', 'oauth_google', 'oauth_github', etc.
    credential_hash TEXT, -- 密码哈希或OAuth token
    oauth_provider_id VARCHAR(100), -- OAuth提供商ID
    oauth_user_id VARCHAR(255), -- OAuth用户ID
    is_primary BOOLEAN NOT NULL DEFAULT false, -- 是否为主要认证方式
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, credential_type, oauth_provider_id)
);
```

### 3. 用户会话表 (user_sessions)
```sql
CREATE TABLE user_sessions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token VARCHAR(255) UNIQUE NOT NULL, -- JWT token或session token
    refresh_token VARCHAR(255) UNIQUE, -- 刷新token
    device_info TEXT, -- 设备信息
    ip_address INET,
    user_agent TEXT,
    expires_at TIMESTAMP NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### 4. OAuth提供商表 (oauth_providers)
```sql
CREATE TABLE oauth_providers (
    id SERIAL PRIMARY KEY,
    provider_name VARCHAR(100) UNIQUE NOT NULL, -- 'google', 'github', 'facebook', etc.
    client_id VARCHAR(255) NOT NULL,
    client_secret VARCHAR(255) NOT NULL,
    auth_url TEXT NOT NULL,
    token_url TEXT NOT NULL,
    user_info_url TEXT NOT NULL,
    scope VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

## API接口

### 1. 用户注册
```
POST /auth/register
Content-Type: application/json

{
    "email": "user@example.com",
    "username": "username",
    "name": "User Name",
    "password": "password123"
}
```

响应：
```json
{
    "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
    "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
    "token_type": "Bearer",
    "expires_in": 3600,
    "user": {
        "id": 1,
        "email": "user@example.com",
        "username": "username",
        "name": "User Name",
        "avatar_url": null
    }
}
```

### 2. 用户登录
```
POST /auth/login
Content-Type: application/json

{
    "email": "user@example.com",
    "password": "password123"
}
```

响应格式同注册接口。

### 3. 刷新Token
```
POST /auth/refresh
Content-Type: application/json

{
    "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
}
```

### 4. 获取用户信息
```
GET /auth/profile
Authorization: Bearer <access_token>
```

### 5. 用户登出
```
POST /auth/logout
Authorization: Bearer <access_token>
```

响应：
```json
{
  "success": true,
  "message": "Logout successful",
  "data": null,
  "code": null,
  "field": null
}
```

登出操作会：
- 使数据库中的所有用户会话失效（`is_active = false`）
- 清除 Redis 中的用户缓存（`user:{user_id}`）
- 清除 Redis 中的用户资料缓存（`user_profile:{user_id}`）
- 清除 Redis 中的工作空间缓存（`user_workspace:{user_id}`）

详细文档请参见 [LOGOUT_API.md](LOGOUT_API.md)

## 安全特性

1. **密码安全**: 使用bcrypt进行密码哈希，成本因子为12
2. **JWT安全**:
   - Access Token有效期1小时
   - Refresh Token有效期7天
   - 使用UUID作为JWT ID防止重放攻击
3. **会话管理**: 支持多设备登录，可单独使会话失效
4. **数据库安全**: 使用参数化查询防止SQL注入

## 中间件

### 1. 认证中间件 (auth_middleware)
- 验证JWT token
- 从数据库获取用户信息
- 将用户信息注入到请求扩展中

### 2. 可选认证中间件 (optional_auth_middleware)
- 类似认证中间件，但不强制要求token
- 适用于可选登录的功能

## 环境变量配置

创建`.env`文件：
```env
# Database Configuration
DATABASE_URL=postgres://username:password@localhost:5432/rust_backend

# Redis Configuration
REDIS_URL=redis://localhost:6379

# JWT Configuration
JWT_SECRET=your-super-secret-jwt-key-change-this-in-production

# OAuth Configuration (for future use)
GOOGLE_CLIENT_ID=your_google_client_id
GOOGLE_CLIENT_SECRET=your_google_client_secret
GITHUB_CLIENT_ID=your_github_client_id
GITHUB_CLIENT_SECRET=your_github_client_secret
```

## 部署步骤

1. **安装依赖**:
   ```bash
   cargo build
   ```

2. **设置数据库**:
   ```bash
   # 创建数据库
   createdb rust_backend

   # 运行迁移
   diesel migration run
   ```

3. **启动服务**:
   ```bash
   cargo run
   ```

## OAuth集成计划

系统已为OAuth集成预留了扩展空间：

1. **数据库表**: `oauth_providers`表存储OAuth提供商配置
2. **认证表**: `user_credentials`表支持多种认证方式
3. **路由**: 预留了OAuth授权和回调路由
4. **配置**: 环境变量中预留了OAuth配置项

### 支持的OAuth提供商
- Google OAuth 2.0
- GitHub OAuth 2.0
- Facebook OAuth 2.0 (可扩展)

## 测试

### 使用curl测试API

1. **注册用户**:
   ```bash
   curl -X POST http://localhost:8000/auth/register \
     -H "Content-Type: application/json" \
     -d '{
       "email": "test@example.com",
       "username": "testuser",
       "name": "Test User",
       "password": "password123"
     }'
   ```

2. **登录**:
   ```bash
   curl -X POST http://localhost:8000/auth/login \
     -H "Content-Type: application/json" \
     -d '{
       "email": "test@example.com",
       "password": "password123"
     }'
   ```

3. **获取用户信息**:
   ```bash
   curl -X GET http://localhost:8000/auth/profile \
     -H "Authorization: Bearer <your_access_token>"
   ```

4. **登出**:
   ```bash
   curl -X POST http://localhost:8000/auth/logout \
     -H "Authorization: Bearer <your_access_token>"
   ```

5. **运行登出演示示例**:
   ```bash
   cargo run --example logout_demo
   ```

## 注意事项

1. 生产环境中请更改JWT_SECRET
2. 建议使用HTTPS
3. 定期清理过期的会话记录
4. 考虑实现rate limiting防止暴力破解
5. 监控登录失败次数，实现账户锁定机制