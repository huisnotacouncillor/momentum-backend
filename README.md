# Momentum Backend

<p align="center">
  <strong>基于 Rust 构建的高性能团队协作后端系统</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2024-orange?style=flat-square&logo=rust" alt="Rust 2024">
  <img src="https://img.shields.io/badge/Axum-0.6-blue?style=flat-square" alt="Axum">
  <img src="https://img.shields.io/badge/PostgreSQL-15-blue?style=flat-square&logo=postgresql" alt="PostgreSQL">
  <img src="https://img.shields.io/badge/Redis-7-red?style=flat-square&logo=redis" alt="Redis">
  <img src="https://img.shields.io/badge/WebSocket-Realtime-green?style=flat-square" alt="WebSocket">
  <img src="https://img.shields.io/badge/Docker-Ready-2496ED?style=flat-square&logo=docker" alt="Docker">
</p>

## 📖 项目简介

Momentum Backend 是一个功能完整、性能卓越的团队协作后端系统，使用 Rust 语言和现代化技术栈构建。项目采用 Axum Web 框架、Diesel ORM、Redis 缓存等技术，提供了丰富的 RESTful API 和 WebSocket 实时通信能力，支持项目管理、任务追踪、团队协作等核心功能。

### ✨ 项目亮点

- 🚀 **高性能**：基于 Rust 和 Tokio 异步运行时，支持高并发场景
- 🔄 **实时通信**：完整的 WebSocket 支持，包含命令系统和事件订阅
- 🛡️ **安全可靠**：JWT 认证、输入验证、速率限制等多层安全机制
- 📊 **性能优化**：Redis 缓存、连接池优化、登录性能优化
- 🔧 **易于部署**：提供 Docker 和 Docker Compose 支持，一键部署
- 📚 **文档完善**：详细的 API 文档、示例代码和架构文档
- 🧪 **测试充分**：包含单元测试、集成测试和压力测试工具
- 🎯 **功能丰富**：支持工作区、项目、任务、团队、标签等完整功能

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
- WebSocket 命令系统（实时 CRUD 操作）
- 在线用户状态管理
- 消息广播与点对点通信
- 心跳检测机制
- 订阅/发布机制

### 高级功能
- **速率限制**：防止 API 滥用，支持基于用户的请求频率控制
- **连接监控**：实时监控 WebSocket 连接状态和性能指标
- **事件系统**：业务事件的发布和订阅机制
- **批量处理**：支持批量操作标签等资源
- **重试与超时**：自动重试失败的操作，超时控制
- **安全机制**：JWT 认证、CORS 配置、输入验证

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
- `POST /ws/cleanup` - 清理过期连接

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
   ```bash
   # 复制示例配置文件
   cp env.example .env
   # 然后根据需要修改 .env 文件
   ```

   主要环境变量：
   ```env
   # 数据库配置
   DATABASE_URL=postgres://postgres:postgres@localhost:5434/rust-backend

   # Redis 配置
   REDIS_URL=redis://127.0.0.1:6379/

   # JWT 配置
   JWT_SECRET=your-super-secret-jwt-key-change-this-in-production

   # 服务器配置
   HOST=127.0.0.1
   PORT=8000

   # 日志级别
   RUST_LOG=info
   ```

   详细配置说明请参考 `env.example` 文件。

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

### WebSocket 命令系统

项目支持通过 WebSocket 进行实时 CRUD 操作，所有命令都采用统一的格式：

```json
{
  "type": "command_name",
  "data": { ... },
  "request_id": "optional-request-id"
}
```

#### 标签命令（Labels）
- `create_label` - 创建标签
- `update_label` - 更新标签
- `delete_label` - 删除标签
- `query_labels` - 查询标签
- `batch_create_labels` - 批量创建标签
- `batch_update_labels` - 批量更新标签
- `batch_delete_labels` - 批量删除标签

#### 团队命令（Teams）
- `create_team` - 创建团队
- `update_team` - 更新团队
- `delete_team` - 删除团队
- `query_teams` - 查询团队
- `add_team_member` - 添加团队成员
- `update_team_member` - 更新团队成员
- `remove_team_member` - 移除团队成员
- `list_team_members` - 列出团队成员

#### 工作区命令（Workspaces）
- `create_workspace` - 创建工作区
- `update_workspace` - 更新工作区
- `delete_workspace` - 删除工作区
- `get_current_workspace` - 获取当前工作区
- `invite_workspace_member` - 邀请工作区成员
- `accept_invitation` - 接受邀请
- `query_workspace_members` - 查询工作区成员

#### 项目命令（Projects）
- `create_project` - 创建项目
- `update_project` - 更新项目
- `delete_project` - 删除项目
- `query_projects` - 查询项目

#### 任务命令（Issues）
- `create_issue` - 创建任务
- `update_issue` - 更新任务
- `delete_issue` - 删除任务
- `query_issues` - 查询任务
- `get_issue` - 获取任务详情

#### 项目状态命令（Project Statuses）
- `create_project_status` - 创建项目状态
- `update_project_status` - 更新项目状态
- `delete_project_status` - 删除项目状态
- `query_project_statuses` - 查询项目状态
- `get_project_status_by_id` - 根据 ID 获取项目状态

#### 用户命令（User）
- `update_profile` - 更新用户资料

#### 连接管理命令
- `subscribe` - 订阅主题
- `unsubscribe` - 取消订阅
- `get_connection_info` - 获取连接信息
- `ping` - 心跳检测

#### 命令示例

创建标签：
```json
{
  "type": "create_label",
  "data": {
    "name": "Bug",
    "color": "#ff0000",
    "level": "issue"
  },
  "request_id": "req-123"
}
```

查询任务：
```json
{
  "type": "query_issues",
  "filters": {
    "project_id": "project-uuid",
    "status": "in_progress"
  },
  "request_id": "req-456"
}
```

订阅主题：
```json
{
  "type": "subscribe",
  "topics": ["issues", "projects", "workspace:workspace-uuid"],
  "request_id": "req-789"
}
```

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
│   ├── unified_manager.rs # 统一连接管理器
│   ├── handler.rs         # 消息处理器
│   ├── auth.rs            # WebSocket 认证
│   ├── security.rs        # 安全机制
│   ├── rate_limiter.rs    # 速率限制
│   ├── monitoring.rs      # 连接监控
│   ├── batch_processor.rs # 批量处理器
│   ├── retry_timeout.rs   # 重试与超时
│   ├── commands/          # 命令系统
│   │   ├── types.rs       # 命令类型定义
│   │   ├── handler.rs     # 命令处理器
│   │   ├── labels.rs      # 标签命令
│   │   ├── teams.rs       # 团队命令
│   │   ├── workspaces.rs  # 工作区命令
│   │   ├── projects.rs    # 项目命令
│   │   ├── issues.rs      # 任务命令
│   │   └── user.rs        # 用户命令
│   ├── events/            # 事件系统
│   │   ├── types.rs       # 事件类型
│   │   ├── handlers.rs    # 事件处理器
│   │   └── business.rs    # 业务事件
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
```

### 运行示例

项目包含丰富的示例代码，涵盖各种功能演示：

#### 基础示例
```bash
# 简单示例
cargo run --example simple

# 测试数据库模式
cargo run --example test_schema
```

#### 认证与性能
```bash
# 登录性能测试
cargo run --example login_performance_test

# 详细登录性能分析
cargo run --example detailed_login_performance

# Bcrypt 性能测试
cargo run --example bcrypt_performance_test

# 测试优化后的登录
cargo run --example test_optimized_login

# 登录性能验证
cargo run --example login_performance_validation

# 令牌自动续期演示
cargo run --example token_auto_renewal_demo

# 通用性能测试
cargo run --example performance_test
```

#### WebSocket 相关
```bash
# WebSocket 演示
cargo run --example unified_websocket_demo

# Issue WebSocket 演示
cargo run --example issues_websocket_demo

# WebSocket 安全演示
cargo run --example websocket_security_demo
```

#### 业务功能
```bash
# 用户资料 API 演示
cargo run --example profile_api_demo

# 工作区切换演示
cargo run --example workspace_switching_demo

# 项目和状态演示
cargo run --example project_with_available_statuses_demo

# Issue 状态流转演示
cargo run --example issue_transitions_demo

# Issue 与分配人演示
cargo run --example issues_with_assignee_demo

# Issue 与团队演示
cargo run --example issues_with_team_demo

# 工作流演示
cargo run --example workflow_demo

# 评论功能演示
cargo run --example comment_demo

# 资源 URL 演示
cargo run --example asset_url_demo

# 统一 API 演示
cargo run --example unified_api_demo
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

## 🔒 高级功能详解

### 速率限制（Rate Limiting）

系统实现了基于用户的智能速率限制，防止 API 滥用：

- **令牌桶算法**：平滑的流量控制
- **用户级限制**：每个用户独立的速率限制
- **可配置阈值**：支持通过环境变量配置限制参数
- **WebSocket 支持**：WebSocket 命令也支持速率限制

配置示例：
```env
RATE_LIMIT_PER_SECOND=10
RATE_LIMIT_WINDOW=60
```

### 连接监控（Monitoring）

实时监控 WebSocket 连接状态和性能指标：

- **连接统计**：实时追踪在线用户数、总连接数
- **性能指标**：监控消息发送速率、延迟等
- **健康检查**：自动检测和清理僵尸连接
- **管理接口**：通过 HTTP API 查询连接状态

查询连接统计：
```bash
curl http://localhost:8000/ws/stats
```

### 事件系统（Events）

灵活的事件发布订阅机制：

- **业务事件**：Issue 创建、更新、删除等业务事件
- **系统事件**：用户上线、下线等系统事件
- **事件路由**：基于主题的事件订阅和分发
- **实时推送**：事件自动通过 WebSocket 推送给订阅者

订阅事件示例：
```json
{
  "type": "subscribe",
  "topics": ["issues", "projects", "workspace:uuid"],
  "request_id": "req-123"
}
```

### 批量处理（Batch Operations）

提高效率的批量操作支持：

- **批量创建标签**：一次请求创建多个标签
- **批量更新**：批量更新多个资源
- **批量删除**：批量删除多个资源
- **事务保证**：批量操作保证原子性

批量创建标签示例：
```json
{
  "type": "batch_create_labels",
  "data": [
    {"name": "Bug", "color": "#ff0000", "level": "issue"},
    {"name": "Feature", "color": "#00ff00", "level": "issue"}
  ]
}
```

### 重试与超时（Retry & Timeout）

增强系统可靠性：

- **自动重试**：失败操作自动重试，支持指数退避
- **超时控制**：防止长时间阻塞操作
- **幂等性保证**：重试操作保证幂等性
- **错误恢复**：智能错误处理和恢复机制

### 安全机制（Security）

多层安全保护：

- **JWT 认证**：所有 API 和 WebSocket 连接都需要 JWT 认证
- **Token 验证**：严格的 token 签名和过期验证
- **CORS 配置**：可配置的跨域资源共享策略
- **输入验证**：使用 validator 进行输入数据验证
- **SQL 注入防护**：使用 Diesel ORM 预防 SQL 注入
- **连接清理**：自动清理过期和无效连接

### 缓存策略（Caching）

智能缓存提升性能：

- **Redis 缓存**：热点数据缓存
- **缓存失效**：自动缓存失效和更新
- **缓存预热**：应用启动时预加载常用数据
- **缓存穿透防护**：防止缓存穿透攻击

## 🚀 部署

### Docker Compose 部署（推荐）

使用 Docker Compose 一键启动完整环境（包括 PostgreSQL、Redis 和后端服务）：

```bash
# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f backend

# 停止所有服务
docker-compose down

# 停止并删除数据卷
docker-compose down -v
```

### Docker 独立部署

如果你已有数据库和 Redis，可以单独构建和运行后端：

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

生产环境建议配置以下环境变量：

```env
# 数据库连接
DATABASE_URL=postgres://user:password@host:port/database

# Redis 连接
REDIS_URL=redis://host:port

# JWT 密钥（务必使用强密钥）
JWT_SECRET=your-super-secret-jwt-key-for-production

# 日志级别
RUST_LOG=info

# 服务器配置
HOST=0.0.0.0
PORT=8000

# WebSocket 配置
WS_MAX_CONNECTIONS=10000
WS_CONNECTION_TIMEOUT=300

# 性能配置
DB_POOL_SIZE=20
REDIS_POOL_SIZE=10

# 安全配置
ENABLE_CORS=true
CORS_ALLOWED_ORIGINS=https://yourdomain.com
```

### 数据库迁移

在部署前需要运行数据库迁移：

```bash
# 在容器中运行迁移
docker exec momentum-backend diesel migration run

# 或者在本地运行
diesel migration run
```

## 📚 文档

### 核心文档
- [API 响应设计](docs/api/API_RESPONSE_DESIGN.md) - API 响应格式和设计规范
- [认证系统说明](docs/auth/AUTH_README.md) - 系统认证机制的详细说明
- [WebSocket 使用指南](docs/websocket/WEBSOCKET_README.md) - WebSocket 功能的完整使用指南
- [WebSocket 实现总结](docs/websocket/WEBSOCKET_IMPLEMENTATION_SUMMARY.md) - WebSocket 功能的实现总结
- [WebSocket 安全实现](docs/websocket/WEBSOCKET_SECURITY.md) - WebSocket 安全机制详解
- [登录性能优化](docs/LOGIN_PERFORMANCE_OPTIMIZATION.md) - 登录性能优化方案

### 数据库文档
- [Momentum 模式实现](docs/database/MOMENTUM_SCHEMA_IMPLEMENTATION.md) - 数据库 schema 设计与实现
- [模型重构总结](docs/database/MODELS_REFACTORING_SUMMARY.md) - 数据模型重构的总结
- [关系模型总结](docs/database/RELATIONSHIP_MODEL_SUMMARY.md) - 数据库关系模型的总结
- [工作区切换优化](docs/database/SWITCH_WORKSPACE_OPTIMIZATION.md) - 工作区切换的数据库优化

### API 文档
- [工作区切换 API](docs/api/WORKSPACE_SWITCHING_API.md) - 工作区切换功能的 API 设计
- [项目 API 实现](docs/api/PROJECT_API_IMPLEMENTATION.md) - 项目相关 API 的实现细节
- [用户资料 API 更新](docs/api/PROFILE_API_UPDATED.md) - 用户资料相关 API 的更新说明
- [Issue 状态流转 API](docs/api/ISSUE_TRANSITIONS_API.md) - Issue 状态流转的 API 设计

### WebSocket 文档
- [WebSocket 功能清单](docs/websocket/WEBSOCKET_CHECKLIST.md) - WebSocket 功能开发清单
- [初始数据特性](docs/websocket/INITIAL_DATA_FEATURE.md) - WebSocket 初始数据加载特性
- [Issues WebSocket 实现](docs/websocket/ISSUES_WEBSOCKET_IMPLEMENTATION.md) - Issues 的 WebSocket 实现
- [安全实现总结](docs/websocket/SECURITY_IMPLEMENTATION_SUMMARY.md) - WebSocket 安全实现总结

### 其他文档
- [资源 URL 实现](docs/ASSETS_URL_IMPLEMENTATION.md) - 资源 URL 处理实现
- [评论功能](docs/comment_feature.md) - 评论功能的实现说明
- [用户资料 API 资源更新](docs/PROFILE_API_ASSETS_UPDATE.md) - 用户资料 API 的资源更新

### 完整文档索引
查看 [docs/INDEX.md](docs/INDEX.md) 获取所有文档的完整列表。

## 🏗️ 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                        客户端应用                             │
│                  (Web, Mobile, Desktop)                     │
└────────────────┬──────────────────┬─────────────────────────┘
                 │                  │
                 │ HTTP/REST        │ WebSocket
                 │                  │
┌────────────────▼──────────────────▼─────────────────────────┐
│                     Axum Web 服务器                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Middleware 中间件层                      │  │
│  │  - JWT 认证  - CORS  - 日志  - 速率限制              │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                  路由层 (Routes)                      │  │
│  │  - Auth  - Workspaces  - Projects  - Issues  - Teams │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │               业务逻辑层 (Services)                   │  │
│  │  - 认证服务  - 项目服务  - 任务服务  - 团队服务      │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌────────────────────┬─────────────────────────────────┐  │
│  │  WebSocket 管理器   │    数据访问层 (Repositories)    │  │
│  │  - 连接管理         │    - Diesel ORM                 │  │
│  │  - 命令处理         │    - 数据模型                   │  │
│  │  - 事件分发         │    - 数据库查询                 │  │
│  └────────────────────┴─────────────────────────────────┘  │
└────────┬─────────────────────────────────────┬─────────────┘
         │                                     │
         │                                     │
┌────────▼──────────┐              ┌──────────▼─────────────┐
│   Redis 缓存      │              │   PostgreSQL 数据库     │
│  - 会话缓存       │              │  - 用户数据             │
│  - 热点数据       │              │  - 工作区数据           │
│  - 速率限制计数   │              │  - 项目和任务数据       │
└───────────────────┘              └────────────────────────┘
```

## 🔗 快速链接

### 开发资源
- [快速开始](#快速开始) - 开始使用 Momentum Backend
- [API 端点](#-api-端点) - 查看所有可用的 API
- [WebSocket 通信](#-websocket-实时通信) - 实时通信文档
- [示例代码](#运行示例) - 查看示例代码
- [文档中心](#-文档) - 完整的项目文档

### 部署与运维
- [Docker 部署](#-部署) - 使用 Docker 部署
- [环境配置](env.example) - 环境变量配置
- [数据库迁移](#数据库操作) - 数据库迁移指南

### 测试与工具
- [运行测试](#运行测试) - 测试指南
- [WebSocket 测试工具](#websocket-测试工具) - WebSocket 压力测试
- [性能优化](#-性能特性) - 性能优化方案

## 📝 更新日志

### 最新特性

- ✅ **WebSocket 命令系统**：支持通过 WebSocket 进行实时 CRUD 操作
- ✅ **事件订阅系统**：灵活的事件发布订阅机制
- ✅ **速率限制**：防止 API 滥用的智能速率限制
- ✅ **连接监控**：实时监控 WebSocket 连接状态
- ✅ **批量操作**：支持批量创建、更新、删除资源
- ✅ **Docker 支持**：完整的 Docker 和 Docker Compose 配置
- ✅ **安全增强**：多层安全防护机制
- ✅ **性能优化**：登录性能优化、缓存策略优化

## ❓ 常见问题

### 如何开始开发？

1. 克隆项目并安装依赖
2. 配置环境变量（复制 `env.example` 为 `.env`）
3. 启动数据库和 Redis
4. 运行数据库迁移：`diesel migration run`
5. 启动服务：`cargo run`

### 如何运行测试？

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test --test integration_tests
```

### 如何使用 WebSocket？

查看 [WebSocket 实时通信](#-websocket-实时通信) 部分，或运行示例：

```bash
cargo run --example unified_websocket_demo
```

### 如何部署到生产环境？

推荐使用 Docker Compose：

```bash
docker-compose up -d
```

详细部署指南请参考 [部署](#-部署) 部分。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 来改进项目！

### 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

[待定]

## 📧 联系方式

如有问题或建议，欢迎通过 Issue 与我们联系。

---

<p align="center">
  Made with ❤️ using Rust
</p>