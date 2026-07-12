# ASSETS_URL 环境变量实现文档

## 概述

本文档描述了 `ASSETS_URL` 环境变量的实现，该功能主要用于处理用户头像、团队图标等静态资源的 URL 构建。

## 功能特性

- ✅ 支持通过环境变量 `ASSETS_URL` 配置资源基础 URL
- ✅ 提供通用的资源 URL 处理工具 `AssetUrlHelper`
- ✅ 自动处理外部链接和内部路径
- ✅ 为所有相关模型提供便捷的 URL 处理方法
- ✅ 支持多种资源类型（头像、团队图标、项目图标、附件等）
- ✅ 包含完整的单元测试

## 环境变量配置

### ASSETS_URL

用于配置静态资源的基础 URL。

**默认值**: `http://localhost:8000/assets`

**示例**:
```bash
# 使用 CDN
export ASSETS_URL="https://cdn.example.com/assets"

# 使用本地静态文件服务
export ASSETS_URL="http://localhost:3000/static"

# 使用相对路径（不推荐）
export ASSETS_URL="/assets"
```

## 核心组件

### 1. AssetsConfig 结构体

```rust
#[derive(Clone, Debug)]
pub struct AssetsConfig {
    pub base_url: String,
}
```

### 2. AssetUrlHelper 工具类

提供通用的资源 URL 处理方法：

```rust
pub struct AssetUrlHelper {
    base_url: String,
}
```

#### 主要方法

- `new(assets_config: &AssetsConfig)` - 创建实例
- `build_url(path: &str)` - 构建完整 URL
- `build_avatar_url(filename: &str)` - 构建头像 URL
- `build_team_icon_url(filename: &str)` - 构建团队图标 URL
- `build_project_icon_url(filename: &str)` - 构建项目图标 URL
- `build_attachment_url(filename: &str)` - 构建附件 URL
- `is_external_url(url: &str)` - 检查是否为外部链接
- `process_url(url: &str)` - 处理 URL（外部链接直接返回，内部路径构建完整 URL）

## 使用方法

### 1. 基本使用

```rust
use rust_backend::config::Config;
use rust_backend::utils::AssetUrlHelper;

// 加载配置
let config = Config::from_env()?;

// 创建资源 URL 处理工具
let asset_helper = AssetUrlHelper::new(&config.assets());

// 构建头像 URL
let avatar_url = asset_helper.build_avatar_url("user123.jpg");
// 结果: "http://localhost:8000/assets/avatars/user123.jpg"

// 构建团队图标 URL
let team_icon_url = asset_helper.build_team_icon_url("team456.png");
// 结果: "http://localhost:8000/assets/team-icons/team456.png"
```

### 2. 在用户模型中使用

```rust
use rust_backend::db::models::auth::User;

let user = User {
    // ... 其他字段
    avatar_url: Some("avatars/user123.jpg".to_string()),
    // ... 其他字段
};

// 获取处理后的头像 URL
if let Some(processed_avatar_url) = user.get_processed_avatar_url(&asset_helper) {
    println!("用户头像: {}", processed_avatar_url);
}
```

### 3. 在团队模型中使用

```rust
use rust_backend::db::models::team::Team;

let team = Team {
    // ... 其他字段
    icon_url: Some("team-icons/dev-team.png".to_string()),
    // ... 其他字段
};

// 获取处理后的图标 URL
if let Some(processed_icon_url) = team.get_processed_icon_url(&asset_helper) {
    println!("团队图标: {}", processed_icon_url);
}
```

### 4. 处理外部链接

```rust
// 外部链接会直接返回，不会被处理
let external_url = "https://gravatar.com/avatar/user123.jpg";
let processed_url = asset_helper.process_url(external_url);
// 结果: "https://gravatar.com/avatar/user123.jpg"

// 内部路径会被构建为完整 URL
let internal_path = "avatars/user123.jpg";
let processed_url = asset_helper.process_url(internal_path);
// 结果: "http://localhost:8000/assets/avatars/user123.jpg"
```

## 支持的模型

以下模型都添加了相应的 URL 处理方法：

### 用户相关模型
- `User` - `get_processed_avatar_url()`
- `AuthUser` - `get_processed_avatar_url()`
- `UserBasicInfo` - `get_processed_avatar_url()`
- `UserProfile` - `get_processed_avatar_url()`

### 团队相关模型
- `Team` - `get_processed_icon_url()`
- `TeamInfo` - `get_processed_icon_url()`
- `TeamBasicInfo` - `get_processed_icon_url()`
- `TeamWithMembers` - `get_processed_icon_url()`

## 资源路径约定

为了保持一致性，建议使用以下路径约定：

- 用户头像: `avatars/{filename}`
- 团队图标: `team-icons/{filename}`
- 项目图标: `project-icons/{filename}`
- 附件文件: `attachments/{filename}`
- 其他资源: `{category}/{filename}`

## 测试

运行示例程序：

```bash
cargo run --example asset_url_demo
```

运行单元测试：

```bash
cargo test utils::asset_url
```

## 部署建议

### 开发环境
```bash
export ASSETS_URL="http://localhost:8000/assets"
```

### 生产环境
```bash
export ASSETS_URL="https://cdn.yourdomain.com/assets"
```

### Docker 环境
```dockerfile
ENV ASSETS_URL=https://cdn.yourdomain.com/assets
```

## 注意事项

1. **外部链接**: 如果 `avatar_url` 或 `icon_url` 已经是完整的 HTTP/HTTPS URL，系统会直接使用，不会进行任何处理。

2. **路径处理**: 内部路径会自动添加 `ASSETS_URL` 前缀，并确保路径格式正确。

3. **默认值**: 如果不设置 `ASSETS_URL` 环境变量，系统会使用默认值 `http://localhost:8000/assets`。

4. **向后兼容**: 现有的代码不需要修改，新的 URL 处理方法是在原有字段基础上的增强功能。

## 未来扩展

可以考虑添加以下功能：

- 支持多个 CDN 域名（负载均衡）
- 支持资源版本控制
- 支持资源压缩和优化
- 支持资源缓存策略
- 支持动态资源生成（如头像占位符）
