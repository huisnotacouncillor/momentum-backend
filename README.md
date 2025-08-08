# Momentum Backend

Momentum Backend 是一个基于 Rust 构建的高性能后端系统，使用 Axum、Diesel 和 Redis 等技术栈，为现代化团队协作应用提供支持。

## 功能特性

- 用户认证和授权系统
- 工作区（Workspace）管理
- 项目（Project）和任务（Issue）管理
- 实时通信（WebSocket）
- 标签（Label）系统
- 团队（Team）协作
- 评论系统
- 数据缓存（Redis）

## 技术栈

- **框架**: Axum web 框架
- **数据库**: PostgreSQL + Diesel ORM
- **缓存**: Redis
- **异步运行时**: Tokio
- **序列化**: Serde
- **日志**: tracing
- **WebSocket**: tokio-tungstenite

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

## 项目结构

```
src/
├── bin/          # 可执行文件
├── cache/        # 缓存相关模块
├── db/           # 数据库模型和访问层
├── middleware/   # 中间件
├── routes/       # API 路由
├── websocket/    # WebSocket 处理逻辑
├── config.rs     # 配置处理
└── main.rs       # 主程序入口
```

## 文档

有关 API 设计、数据库模型和系统架构的详细信息，请查看 [docs](./docs/INDEX.md) 目录。

## 许可证

[待定]