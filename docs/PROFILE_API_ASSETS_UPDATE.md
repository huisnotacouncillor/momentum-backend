# GET /profile 接口资源 URL 处理更新

## 概述

本文档描述了 `GET /profile` 接口的更新，该接口现在支持使用 `ASSETS_URL` 环境变量来处理用户头像和团队图标的 URL。

## 更新内容

### 1. 函数签名更新

**之前**:
```rust
pub async fn get_profile(
    State(pool): State<Arc<DbPool>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse
```

**现在**:
```rust
pub async fn get_profile(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse
```

### 2. 资源 URL 处理

接口现在使用 `AssetUrlHelper` 来处理所有资源 URL：

```rust
// 创建资源 URL 处理工具
let asset_helper = crate::utils::AssetUrlHelper::new(&state.config.assets());
```

### 3. 用户头像处理

**之前**:
```rust
avatar_url: user.avatar_url,
```

**现在**:
```rust
avatar_url: user.get_processed_avatar_url(&asset_helper),
```

### 4. 团队图标处理

**之前**:
```rust
icon_url: team.icon_url,
```

**现在**:
```rust
icon_url: team.get_processed_icon_url(&asset_helper),
```

## 功能特性

- ✅ 自动处理用户头像 URL
- ✅ 自动处理团队图标 URL
- ✅ 支持外部链接（直接返回）
- ✅ 支持内部路径（添加 ASSETS_URL 前缀）
- ✅ 向后兼容现有数据

## API 响应格式

### 成功响应

```json
{
  "success": true,
  "message": "Profile retrieved successfully",
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "username": "username",
    "name": "用户姓名",
    "avatar_url": "http://localhost:8000/assets/avatars/user123.jpg",
    "current_workspace_id": "550e8400-e29b-41d4-a716-446655440001",
    "workspaces": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "name": "工作空间名称",
        "url_key": "workspace-key"
      }
    ],
    "teams": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440002",
        "name": "团队名称",
        "team_key": "TEAM",
        "description": "团队描述",
        "icon_url": "http://localhost:8000/assets/team-icons/team123.png",
        "is_private": false,
        "role": "member"
      }
    ]
  }
}
```

## URL 处理示例

### 用户头像处理

| 原始值 | 处理后结果 |
|--------|------------|
| `"avatars/user123.jpg"` | `"http://localhost:8000/assets/avatars/user123.jpg"` |
| `"https://gravatar.com/avatar/user123.jpg"` | `"https://gravatar.com/avatar/user123.jpg"` |
| `null` | `null` |

### 团队图标处理

| 原始值 | 处理后结果 |
|--------|------------|
| `"team-icons/team123.png"` | `"http://localhost:8000/assets/team-icons/team123.png"` |
| `"https://cdn.example.com/icons/team123.png"` | `"https://cdn.example.com/icons/team123.png"` |
| `null` | `null` |

## 环境变量配置

### ASSETS_URL

用于配置静态资源的基础 URL。

**默认值**: `http://localhost:8000/assets`

**示例**:
```bash
# 开发环境
export ASSETS_URL="http://localhost:8000/assets"

# 生产环境
export ASSETS_URL="https://cdn.yourdomain.com/assets"

# Docker 环境
ENV ASSETS_URL=https://cdn.yourdomain.com/assets
```

## 测试

运行示例程序：

```bash
cargo run --example profile_api_demo
```

## 向后兼容性

- ✅ 现有的数据库数据无需修改
- ✅ 外部链接继续正常工作
- ✅ 内部路径会自动添加正确的前缀
- ✅ 空值（null）保持不变

## 注意事项

1. **外部链接**: 如果 `avatar_url` 或 `icon_url` 已经是完整的 HTTP/HTTPS URL，系统会直接使用，不会进行任何处理。

2. **路径处理**: 内部路径会自动添加 `ASSETS_URL` 前缀，并确保路径格式正确。

3. **默认值**: 如果不设置 `ASSETS_URL` 环境变量，系统会使用默认值 `http://localhost:8000/assets`。

4. **性能**: URL 处理是轻量级操作，不会显著影响接口性能。

## 相关文件

- `src/routes/auth.rs` - 主要的接口实现
- `src/utils/asset_url.rs` - 资源 URL 处理工具
- `src/config.rs` - 配置管理
- `examples/profile_api_demo.rs` - 使用示例
