# 认证与授权（Auth）

> 对应代码：`momentum_api/src/routes/{auth,oauth}.rs`、`momentum_api/src/middleware/auth.rs`、`momentum_core/src/services/{auth_service,jwt,oauth_service}.rs`
> 协议：JWT（Access Token + Refresh Token）

历史版本（2025-10 三篇拆分文档）已归档到 `docs/_archive/2025/auth/`。

---

## 📚 文档导航

| 章节 | 内容 |
|---|---|
| **[§1 端点速览](#1-端点速览)** | HTTP 路由清单 |
| **[§2 注册 / 登录](#2-注册--登录)** | 用户生命周期 |
| **[§3 Token 管理](#3-token-管理)** | JWT/Refresh 机制、自动续期 |
| **[§4 用户资料](#4-用户资料)** | profile + avatar |
| **[§5 登出](#5-登出)** | 多设备会话失效 + 缓存清理 |
| **[§6 OAuth](#6-oauth-预留)** | 第三方登录（已留入口） |
| **[§7 安全要点](#7-安全要点)** | 密码哈希、会话管理、已知风险 |
| **[§8 RBAC](#8-rbac--工作区权限)** | 见 ADR-0005 |

---

## 1. 端点速览

| 方法 | 路径 | 认证 | 用途 |
|---|---|---|---|
| POST | `/auth/register` | 无 | 注册新用户 |
| POST | `/auth/login` | 无 | 用户名 + 密码登录 |
| POST | `/auth/refresh` | 无 | 刷新 access token |
| POST | `/auth/logout` | Bearer | 登出（多设备） |
| GET | `/auth/profile` | Bearer | 当前用户资料 |
| PUT | `/auth/profile` | Bearer | 更新资料 |
| GET | `/auth/oauth/:provider` | 无 | OAuth 跳转 |
| GET | `/auth/oauth/:provider/callback` | 无 | OAuth 回调 |

---

## 2. 注册 / 登录

### 注册

```bash
curl -X POST http://localhost:8000/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "username": "username",
    "name": "User Name",
    "password": "password123"
  }'
```

响应（200 OK）：
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "username": "username",
    "name": "User Name",
    "avatar_url": null,
    "current_workspace_url_key": "my-workspace"  // 新用户引导流程
  }
}
```

`current_workspace_url_key` 在登录/注册响应中返回（2026-Q2 引入），方便前端直接跳转。

### 登录

```bash
curl -X POST http://localhost:8000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}'
```

响应格式同注册。

---

## 3. Token 管理

| Token | 有效期 | 用途 |
|---|---|---|
| Access Token | 1 小时 | API 调用 |
| Refresh Token | 7 天 | 刷新 access token |

### JWT Claims

```json
{
  "sub": "user-uuid",
  "username": "...",
  "email": "...",
  "exp": 1234567890,
  "iat": 1234567890,
  "jti": "unique-jwt-id"
}
```

### 自动续期（客户端）

```javascript
// 401 触发自动 refresh
async function fetchWithAuth(url, options) {
  let token = localStorage.getItem('access_token');
  let res = await fetch(url, {
    ...options,
    headers: { ...options.headers, Authorization: `Bearer ${token}` }
  });
  if (res.status === 401) {
    const refresh = localStorage.getItem('refresh_token');
    const r = await fetch('/auth/refresh', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: refresh })
    });
    const data = await r.json();
    localStorage.setItem('access_token', data.access_token);
    token = data.access_token;
    res = await fetch(url, { ...options, headers: { Authorization: `Bearer ${token}` } });
  }
  return res;
}
```

### Refresh

```bash
curl -X POST http://localhost:8000/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "eyJ0eXAi..."}'
```

⚠️ **当前 Refresh Token 无旋转**（ARCHITECTURE_REVIEW.md §2）：7 天有效期内可重复使用。建议：
- 短期：缩短 refresh token 有效期
- 中期：实现 token 旋转 + 黑名单

---

## 4. 用户资料

```bash
# 获取
curl -H "Authorization: Bearer $TOKEN" http://localhost:8000/auth/profile

# 更新
curl -X PUT http://localhost:8000/auth/profile \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "New Name", "avatar_url": "https://..."}'
```

---

## 5. 登出

### 接口

```bash
curl -X POST http://localhost:8000/auth/logout \
  -H "Authorization: Bearer $TOKEN"
```

### 服务端行为

1. **会话失效**：DB 中该用户所有 `user_sessions.is_active = false`
2. **缓存清理**：Redis 删除
   - `user:{user_id}`
   - `user_profile:{user_id}`
   - `user_workspace:{user_id}`
3. **客户端**：收到 200 后**必须**删除本地 access/refresh token

### 容错

- Redis 失败不阻断登出（DB 会话已失效，缓存自然 TTL 过期）
- 即使登出 API 调用失败，客户端也应当清本地 token

### 多设备登出

当前实现是"一次登出，全设备失效"。未来可支持"选择性登出特定设备"（见 _archive 文档 §后续改进）。

### 客户端最佳实践

```javascript
async function logout() {
  const token = localStorage.getItem('access_token');
  try {
    await fetch('/auth/logout', {
      method: 'POST',
      headers: { Authorization: `Bearer ${token}` }
    });
  } finally {
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    window.location.href = '/login';
  }
}
```

---

## 6. OAuth（预留）

当前 `momentum_api/src/routes/oauth.rs` 提供：
- `GET /auth/oauth/:provider` 跳转
- `GET /auth/oauth/:provider/callback` 回调

数据库已支持 `oauth_providers` + `user_credentials` 表，结构上可接入 Google/GitHub 等。

⚠️ **当前未启用**：OAuth 集成仍属"已留入口但未生产"的阶段（见 _archive 文档 §OAuth 集成计划）。

---

## 7. 安全要点

| 项 | 实现 | 备注 |
|---|---|---|
| 密码哈希 | bcrypt | 成本因子 12 |
| Access Token 有效期 | 1 小时 | |
| Refresh Token 有效期 | 7 天 | ⚠️ 无旋转 |
| JWT ID | UUID v4 | 防重放 |
| 多设备会话 | 支持 | |
| SQL 注入防护 | Diesel 参数化查询 | |
| 中间件 | `auth_middleware` + `optional_auth_middleware` | |

### 已知风险

来自 `docs/architecture/ARCHITECTURE_REVIEW.md`：

- 🔴 Refresh Token 无旋转（7 天可重复用，无撤销）
- 🔴 JWT 默认密钥回退（`AuthConfig::default()` 允许空 JWT_SECRET）
- 🟡 无审计日志（登录、登出、密码修改）
- 🟡 无登录失败计数 / 账号锁定

---

## 8. RBAC & 工作区权限

工作区权限模型（Owner/Admin/Member/Guest）见：

- **`docs/adr/0005-rbac-model.md`** - 决策记录
- **`docs/architecture/ARCHITECTURE_REVIEW.md`** §漏洞 4（RBAC 尚未强制执行）

---

## 9. 测试

```bash
# 单元测试
cargo test auth

# 完整流程
cargo run --example login_with_workspace_demo
cargo run --example logout_demo
cargo run --example token_auto_renewal_demo
```

---

**最后更新**：2026-07-12