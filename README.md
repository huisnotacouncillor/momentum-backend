# Momentum Backend

Momentum Backend 是一个基于 Rust 构建的高性能团队协作后端系统，使用 Axum、Diesel 和 Redis 等技术栈，为现代化项目管理应用提供完整的 API 支持。

## 🚀 核心功能

### 用户认证与授权
- JWT 令牌认证系统
- 用户注册、登录、资料管理
- 密码加密存储（bcrypt）
- 令牌自动续期机制

### 工作区管理
- 多工作区支持
- 工作区成员管理
- 工作区切换优化
- 邀请系统

### 项目管理
- 项目创建与管理
- 项目状态管理
- 项目优先级设置
- 项目路线图支持

### 任务管理
- Issue 创建、更新、删除
- 任务状态流转
- 任务分配与标签
- 任务评论系统

### 团队协作
- 团队创建与管理
- 团队成员权限控制
- 工作流（Workflow）管理
- 工作流状态定义

### 实时通信
- WebSocket 实时消息推送
- 在线用户状态管理
- 消息广播与点对点通信
- 心跳检测机制

### 其他功能
- 标签系统管理
- 评论功能
- 周期（Cycle）管理
- 数据缓存（Redis）
- 资源 URL 处理

## 📡 API 端点

### 认证相关
- `POST /auth/register` - 用户注册
- `POST /auth/login` - 用户登录
- `POST /auth/refresh` - 刷新令牌
- `GET /auth/profile` - 获取用户资料
- `PUT /auth/profile` - 更新用户资料

### 工作区管理
- `GET /workspaces` - 获取工作区列表
- `POST /workspaces` - 创建新工作区
- `GET /workspaces/{id}` - 获取工作区详情
- `PUT /workspaces/{id}` - 更新工作区
- `POST /workspaces/switch` - 切换当前工作区
- `GET /workspaces/{id}/members` - 获取工作区成员

### 项目管理
- `GET /projects` - 获取项目列表
- `POST /projects` - 创建新项目
- `GET /projects/{id}` - 获取项目详情
- `PUT /projects/{id}` - 更新项目
- `DELETE /projects/{id}` - 删除项目

### 任务管理
- `GET /issues` - 获取任务列表
- `POST /issues` - 创建新任务
- `GET /issues/{id}` - 获取任务详情
- `PUT /issues/{id}` - 更新任务
- `DELETE /issues/{id}` - 删除任务
- `POST /issues/{id}/transitions` - 任务状态流转

### 团队管理
- `GET /teams` - 获取团队列表
- `POST /teams` - 创建新团队
- `GET /teams/{id}` - 获取团队详情
- `PUT /teams/{id}` - 更新团队

### 工作流管理
- `GET /workflows` - 获取工作流列表
- `POST /workflows` - 创建新工作流
- `POST /workflows/{id}/states` - 添加工作流状态

### 标签管理
- `GET /labels` - 获取标签列表
- `POST /labels` - 创建新标签
- `PUT /labels/{id}` - 更新标签
- `DELETE /labels/{id}` - 删除标签

### 评论系统
- `GET /comments` - 获取评论列表
- `POST /comments` - 创建新评论
- `PUT /comments/{id}` - 更新评论
- `DELETE /comments/{id}` - 删除评论

### 邀请管理
- `GET /invitations` - 获取邀请列表
- `POST /invitations` - 发送邀请
- `POST /invitations/{id}/accept` - 接受邀请
- `POST /invitations/{id}/decline` - 拒绝邀请

### WebSocket 端点
- `ws://host:8000/ws?token=<JWT>` - WebSocket 连接
- `GET /ws/online` - 获取在线用户列表
- `GET /ws/stats` - 获取连接统计
- `POST /ws/send` - 发送消息给特定用户
- `POST /ws/broadcast` - 广播消息给所有用户

## 🛠 技术栈

- **框架**: Axum web 框架
- **数据库**: PostgreSQL + Diesel ORM
- **缓存**: Redis
- **异步运行时**: Tokio
- **序列化**: Serde
- **日志**: tracing
- **WebSocket**: tokio-tungstenite
- **认证**: JWT + bcrypt
- **验证**: validator
- **错误处理**: thiserror

## 快速开始

### 环境要求

- Rust 2024 edition
- PostgreSQL 数据库
- Redis 服务器

### 安装步骤

1. 克隆项目仓库：
   ```bash
   git clone <repository-url>
   cd momentum_backend
   ```

2. 启动 PostgreSQL 和 Redis 服务

3. 配置环境变量，创建 `.env` 文件：
   ```env
   DATABASE_URL=postgres://postgres:postgres@localhost:5434/rust-backend
   REDIS_URL=redis://127.0.0.1:6379/
   JWT_SECRET=your-super-secret-jwt-key-change-this-in-production
   ```

4. 运行数据库迁移：
   ```bash
   diesel migration run
   ```

5. 启动服务：
   ```bash
   cargo run
   ```

服务将在 `http://127.0.0.1:8000` 上运行，WebSocket 端点位于 `ws://127.0.0.1:8000/ws`。

## 🔌 WebSocket 实时通信

### 连接方式
```javascript
const token = "your_jwt_token_here";
const ws = new WebSocket(`ws://localhost:8000/ws?token=${token}`);
```

### 消息格式
```json
{
  "id": "message-uuid",
  "message_type": "text|notification|system_message|ping|pong|user_joined|user_left|error",
  "data": {
    "content": "消息内容",
    "additional_field": "其他数据"
  },
  "timestamp": "2024-01-01T00:00:00Z",
  "from_user_id": "sender-uuid-optional",
  "to_user_id": "recipient-uuid-optional"
}
```

### 支持的消息类型
- `text` - 普通文本消息
- `notification` - 通知消息
- `system_message` - 系统消息
- `ping/pong` - 心跳检测
- `user_joined/user_left` - 用户状态变更
- `error` - 错误消息

### 管理端点
- `GET /ws/online` - 获取在线用户列表
- `GET /ws/stats` - 获取连接统计信息
- `POST /ws/send` - 发送消息给特定用户
- `POST /ws/broadcast` - 广播消息给所有用户
- `POST /ws/cleanup` - 手动清理过期连接

## 📁 项目结构

```
src/
├── bin/                    # 可执行文件
│   ├── websocket_client.rs    # WebSocket 客户端工具
│   └── websocket_stress_test.rs # WebSocket 压力测试工具
├── cache/                  # 缓存相关模块
│   ├── mod.rs             # 缓存模块入口
│   ├── redis.rs           # Redis 缓存实现
│   └── types.rs           # 缓存类型定义
├── db/                     # 数据库模型和访问层
│   ├── models/            # 数据模型定义
│   │   ├── auth.rs        # 用户认证模型
│   │   ├── issue.rs       # 任务模型
│   │   ├── project.rs     # 项目模型
│   │   ├── team.rs        # 团队模型
│   │   ├── workspace.rs   # 工作区模型
│   │   └── ...            # 其他模型
│   └── repositories/      # 数据访问层
├── middleware/             # 中间件
│   ├── auth.rs            # 认证中间件
│   ├── cors.rs            # CORS 中间件
│   └── logging.rs         # 日志中间件
├── routes/                 # API 路由
│   ├── auth.rs            # 认证路由
│   ├── issues.rs          # 任务路由
│   ├── projects.rs        # 项目路由
│   ├── teams.rs           # 团队路由
│   ├── workspaces.rs      # 工作区路由
│   └── ...                # 其他路由
├── services/               # 业务逻辑层
│   ├── auth_service.rs    # 认证服务
│   ├── issues_service.rs  # 任务服务
│   ├── projects_service.rs # 项目服务
│   └── ...                # 其他服务
├── websocket/              # WebSocket 处理逻辑
│   ├── manager.rs         # 连接管理器
│   ├── handler.rs         # 消息处理器
│   ├── auth.rs            # WebSocket 认证
│   └── tests.rs           # WebSocket 测试
├── validation/             # 数据验证
├── utils/                  # 工具函数
├── config.rs              # 配置处理
├── error.rs               # 错误定义
├── schema.rs              # 数据库模式
└── main.rs                # 主程序入口
```

## 🧪 开发与测试

### 运行测试
```bash
# 运行所有测试
cargo test

# 运行单元测试
cargo test --lib

# 运行集成测试
cargo test --test integration_tests

# 运行 WebSocket 测试
cargo test --test websocket
```

### 开发工具
```bash
# 代码格式化
cargo fmt

# 代码检查
cargo clippy

# 生成文档
cargo doc --open

# 运行示例
cargo run --example simple
```

### 数据库操作
```bash
# 创建新迁移
diesel migration generate migration_name

# 运行迁移
diesel migration run

# 回滚迁移
diesel migration redo

# 重置数据库
diesel database reset
```

### WebSocket 测试工具
项目提供了专门的 WebSocket 测试工具：

```bash
# 运行 WebSocket 客户端
cargo run --bin websocket_client

# 运行压力测试
cargo run --bin websocket_stress_test
```

## 📊 性能特性

- **异步处理**: 基于 Tokio 异步运行时，支持高并发
- **连接池**: 数据库连接池优化，减少连接开销
- **缓存机制**: Redis 缓存热点数据，提升响应速度
- **JWT 优化**: 登录性能优化，支持令牌自动续期
- **WebSocket 优化**: 高效的实时通信，支持大量并发连接

## 🚀 部署

### Docker 部署（推荐）
```bash
# 构建镜像
docker build -t momentum-backend .

# 运行容器
docker run -p 8000:8000 \
  -e DATABASE_URL=postgres://user:pass@host:port/db \
  -e REDIS_URL=redis://host:port \
  -e JWT_SECRET=your-secret-key \
  momentum-backend
```

### 生产环境配置
```env
# 生产环境变量
DATABASE_URL=postgres://user:password@host:port/database
REDIS_URL=redis://host:port
JWT_SECRET=your-super-secret-jwt-key-for-production
RUST_LOG=info
```

## 📚 文档

### 核心文档
- [API 响应设计](docs/api/API_RESPONSE_DESIGN.md) - API 响应格式和设计规范
- [认证系统说明](docs/auth/AUTH_README.md) - 系统认证机制的详细说明
- [WebSocket 实现总结](docs/websocket/WEBSOCKET_IMPLEMENTATION_SUMMARY.md) - WebSocket 功能的实现总结

### 数据库文档
- [Momentum 模式实现](docs/database/MOMENTUM_SCHEMA_IMPLEMENTATION.md) - 数据库 schema 设计与实现
- [模型重构总结](docs/database/MODELS_REFACTORING_SUMMARY.md) - 数据模型重构的总结

### API 文档
- [工作区切换 API](docs/api/WORKSPACE_SWITCHING_API.md) - 工作区切换功能的 API 设计
- [项目 API 实现](docs/api/PROJECT_API_IMPLEMENTATION.md) - 项目相关 API 的实现细节
- [用户资料 API 更新](docs/api/PROFILE_API_UPDATED.md) - 用户资料相关 API 的更新说明

### 完整文档索引
查看 [docs/INDEX.md](docs/INDEX.md) 获取所有文档的完整列表。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 来改进项目。

## 📄 许可证

[待定]