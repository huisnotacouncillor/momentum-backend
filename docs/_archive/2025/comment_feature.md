# Comment 功能实现文档

## 概述

基于Linear的issue comment功能特性，我们实现了一个完整的评论系统，支持丰富的交互功能和现代化的用户体验。

## 功能特性

### 🔧 核心功能
- **基础评论管理**: 创建、读取、更新、删除评论
- **Markdown支持**: 支持富文本格式，包括代码块、链接、列表等
- **嵌套回复**: 支持评论回复，形成讨论线程
- **软删除机制**: 删除的评论不会从数据库中物理删除

### 👥 协作功能
- **@用户提及**: 在评论中@提及团队成员，自动发送通知
- **权限控制**: 用户只能编辑和删除自己的评论
- **作者信息**: 每个评论都包含作者的详细信息

### 📎 附件系统
- **文件上传**: 支持图片、文档等文件附件
- **文件信息**: 记录文件名、大小、MIME类型等元数据
- **URL存储**: 支持外部文件URL引用

### 👍 表情反应
- **多种反应**: 支持点赞、爱心、竖起大拇指等表情反应
- **去重机制**: 同一用户对同一评论的同种反应只能有一个
- **实时统计**: 统计每种反应的数量

## 数据库设计

### 主要表结构

#### comments 表
```sql
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    issue_id UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES users(id),
    content TEXT NOT NULL,
    content_type VARCHAR(20) DEFAULT 'markdown',
    parent_comment_id UUID REFERENCES comments(id) ON DELETE CASCADE,
    is_edited BOOLEAN DEFAULT FALSE,
    is_deleted BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### comment_mentions 表
```sql
CREATE TABLE comment_mentions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    comment_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    mentioned_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### comment_attachments 表
```sql
CREATE TABLE comment_attachments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    comment_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    file_name VARCHAR(255) NOT NULL,
    file_url TEXT NOT NULL,
    file_size BIGINT,
    mime_type VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### comment_reactions 表
```sql
CREATE TABLE comment_reactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    comment_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reaction_type VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(comment_id, user_id, reaction_type)
);
```

## API 接口

### 评论管理

#### 获取评论列表
```http
GET /api/issues/:issue_id/comments?page=1&limit=20&include_deleted=false
```

**响应示例:**
```json
{
  "comments": [
    {
      "comment": {
        "id": "uuid",
        "issue_id": "uuid",
        "author_id": "uuid",
        "content": "这是一个评论",
        "content_type": "markdown",
        "parent_comment_id": null,
        "is_edited": false,
        "is_deleted": false,
        "created_at": "2025-09-11T07:00:00Z",
        "updated_at": "2025-09-11T07:00:00Z"
      },
      "author": {
        "id": "uuid",
        "name": "用户名",
        "email": "user@example.com",
        "avatar_url": "https://example.com/avatar.jpg"
      },
      "mentions": [],
      "attachments": [],
      "reactions": [],
      "replies": []
    }
  ],
  "total": 1,
  "page": 1,
  "limit": 20
}
```

#### 创建评论
```http
POST /api/issues/:issue_id/comments
Content-Type: application/json

{
  "content": "这是一个新评论",
  "content_type": "markdown",
  "parent_comment_id": null,
  "mentions": ["user_uuid_1", "user_uuid_2"],
  "attachments": [
    {
      "file_name": "screenshot.png",
      "file_url": "https://example.com/files/screenshot.png",
      "file_size": 1048576,
      "mime_type": "image/png"
    }
  ]
}
```

#### 更新评论
```http
PUT /api/comments/:comment_id
Content-Type: application/json

{
  "content": "更新后的评论内容",
  "content_type": "markdown"
}
```

#### 删除评论
```http
DELETE /api/comments/:comment_id
```

### 表情反应

#### 添加反应
```http
POST /api/comments/:comment_id/reactions
Content-Type: application/json

{
  "reaction_type": "thumbs_up"
}
```

#### 移除反应
```http
DELETE /api/comments/:comment_id/reactions/:reaction_type
```

## 技术实现

### 后端架构
- **框架**: Rust + Axum
- **数据库**: PostgreSQL + Diesel ORM
- **认证**: JWT Token认证
- **权限**: 基于用户身份的权限控制

### 关键特性
1. **递归查询**: 支持评论回复的嵌套结构
2. **软删除**: 保留数据完整性，支持恢复
3. **索引优化**: 针对常用查询添加数据库索引
4. **类型安全**: 使用Rust的类型系统确保数据安全

### 性能优化
- 分页查询减少数据传输量
- 数据库索引优化查询性能
- 批量操作减少数据库访问次数

## 使用示例

### 运行演示
```bash
cargo run --example comment_demo
```

### 测试API
```bash
# 获取评论列表
curl -H "Authorization: Bearer <token>" \
     "http://localhost:3000/api/issues/issue_id/comments"

# 创建评论
curl -X POST \
     -H "Authorization: Bearer <token>" \
     -H "Content-Type: application/json" \
     -d '{"content":"测试评论","content_type":"markdown"}' \
     "http://localhost:3000/api/issues/issue_id/comments"
```

## 扩展功能

### 未来可能的增强
1. **实时通知**: WebSocket推送评论更新
2. **评论搜索**: 全文搜索评论内容
3. **评论模板**: 预定义评论模板
4. **评论导出**: 导出评论为PDF或其他格式
5. **评论统计**: 评论活跃度统计分析

### 集成建议
1. **通知系统**: 集成邮件、Slack等通知渠道
2. **文件存储**: 集成AWS S3、阿里云OSS等云存储
3. **内容审核**: 集成内容安全检测服务
4. **多语言**: 支持国际化和本地化

## 总结

Comment功能的实现提供了完整的评论系统基础设施，支持现代化的协作需求。通过模块化的设计和RESTful API，可以轻松集成到现有的项目管理系统中，为团队协作提供强大的沟通工具。