# 后端测试方案

> 创建日期: 2026-07-05
> 最后更新: 2026-07-05
> 状态: 待实施

---

## 一、测试分层架构

```
┌─────────────────────────────────────────────────────────┐
│                    集成测试                            │
│        WebSocket + PostgreSQL + 运行中的服务器          │
│              测试命令的完整 CRUD 流程                  │
├─────────────────────────────────────────────────────────┤
│                    单元测试                            │
│               Rust cargo test (库测试)                 │
│          测试序列化/反序列化、handler 逻辑              │
├─────────────────────────────────────────────────────────┤
│                    文档测试                            │
│              测试代码示例的正确性验证                   │
├─────────────────────────────────────────────────────────┤
│                    安全测试                            │
│         SQL 注入、XSS、权限提升、水平越权等             │
├─────────────────────────────────────────────────────────┤
│                    性能/负载测试                        │
│              WebSocket 并发、响应时间、吞吐量           │
└─────────────────────────────────────────────────────────┘
```

---

## 二、单元测试 (Unit Tests)

### 2.1 概述
- **位置**: `momentum_api/src/websocket/tests.rs`
- **运行**: `cargo test --lib websocket_tests`
- **特点**: 无外部依赖，快速执行

### 2.2 已实现的测试

| 测试名称 | 覆盖内容 |
|---------|---------|
| `test_team_commands_serialization` | Team workflow status 命令序列化/反序列化 |
| `test_workspace_commands_serialization` | Workspace 命令序列化 |
| `test_workspace_member_commands_serialization` | Workspace Member 命令序列化 |
| `test_comment_commands_serialization` | Comment 命令序列化 |
| `test_command_type_methods` | 所有新命令的 command_type() 方法 |

### 2.3 待补充测试

#### 2.3.1 Handler 逻辑测试

```rust
// 位置: momentum_api/src/websocket/tests.rs

#[cfg(test)]
mod handler_tests {
    use super::*;
    use crate::websocket::handlers::{handle_command, HandleResult};

    /// 测试 GetTeam handler - 成功路径
    #[test]
    fn test_handle_get_team_success() {
        // Given
        let team_id = Uuid::new_v4();
        let ctx = TestContext::new();

        // 预先在数据库中创建 team
        block_on(async {
            create_test_team(&ctx.pool, team_id).await;
        });

        let cmd = GetTeamCommand { team_id };

        // When
        let result = block_on(handle_command(cmd, &ctx));

        // Then
        match result {
            HandleResult::Response(resp) => {
                assert!(resp.success);
                assert_eq!(resp.data["id"], team_id.to_string());
            }
            HandleResult::Error(e) => panic!("Expected success, got error: {:?}", e),
        }
    }

    /// 测试 GetTeam handler - Team 不存在
    #[test]
    fn test_handle_get_team_not_found() {
        // Given
        let team_id = Uuid::new_v4(); // 不存在的 ID
        let ctx = TestContext::new();
        let cmd = GetTeamCommand { team_id };

        // When
        let result = block_on(handle_command(cmd, &ctx));

        // Then
        match result {
            HandleResult::Error(AppError::NotFound(_)) => {} // 期望的
            _ => panic!("Expected NotFound error"),
        }
    }

    /// 测试 CreateTeamWorkflowStatus - category 验证
    #[test]
    fn test_create_team_workflow_status_invalid_category() {
        // Given
        let cmd = CreateTeamWorkflowStatusCommand {
            name: "Test".to_string(),
            category: "invalid_category".to_string(), // 无效
            color: "#FF0000".to_string(),
            description: None,
            position: 0,
        };
        let ctx = TestContext::new();

        // When
        let result = block_on(handle_command(cmd, &ctx));

        // Then
        match result {
            HandleResult::Error(AppError::ValidationError(msg)) => {
                assert!(msg.contains("category"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    /// 测试 category 有效值
    #[test]
    fn test_create_team_workflow_status_valid_categories() {
        let valid_categories = vec![
            "backlog", "started", "completed", "cancelled"
        ];

        for category in valid_categories {
            let cmd = CreateTeamWorkflowStatusCommand {
                name: "Test".to_string(),
                category: category.to_string(),
                color: "#FF0000".to_string(),
                description: None,
                position: 0,
            };
            let ctx = TestContext::new();
            let result = block_on(handle_command(cmd, &ctx));

            match result {
                HandleResult::Response(_) => {} // 期望成功
                HandleResult::Error(e) => panic!(
                    "Category '{}' should be valid but got error: {:?}", 
                    category, e
                ),
            }
        }
    }

    /// 测试 UpdateWorkspaceMember - 权限验证
    #[test]
    fn test_update_workspace_member_permission() {
        // Given: 非 admin 用户尝试更新其他用户角色
        let ctx = TestContext::with_role(Role::Member);
        let target_user_id = Uuid::new_v4();

        let cmd = UpdateWorkspaceMemberCommand {
            user_id: target_user_id,
            data: UpdateWorkspaceMemberData { role: Some("admin".to_string()) },
        };

        // When
        let result = block_on(handle_command(cmd, &ctx));

        // Then
        match result {
            HandleResult::Error(AppError::PermissionDenied) => {} // 期望
            _ => panic!("Expected PermissionDenied"),
        }
    }
}
```

#### 2.3.2 命令边界条件测试

```rust
#[cfg(test)]
mod boundary_tests {
    use super::*;

    /// 测试空名称
    #[test]
    fn test_create_team_workflow_status_empty_name() {
        let cmd = CreateTeamWorkflowStatusCommand {
            name: "".to_string(),
            category: "backlog".to_string(),
            color: "#FF0000".to_string(),
            description: None,
            position: 0,
        };
        let ctx = TestContext::new();
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(AppError::ValidationError(_)) => {} // 期望
            _ => panic!("Expected ValidationError for empty name"),
        }
    }

    /// 测试超长名称（> 255 字符）
    #[test]
    fn test_create_team_workflow_status_name_too_long() {
        let cmd = CreateTeamWorkflowStatusCommand {
            name: "a".repeat(256),
            category: "backlog".to_string(),
            color: "#FF0000".to_string(),
            description: None,
            position: 0,
        };
        let ctx = TestContext::new();
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(AppError::ValidationError(msg)) => {
                assert!(msg.contains("name") && msg.contains("255"));
            }
            _ => panic!("Expected ValidationError for name too long"),
        }
    }

    /// 测试超长内容（Comment）
    #[test]
    fn test_create_comment_content_too_long() {
        let cmd = CreateCommentCommand {
            content: "a".repeat(100001), // 超过限制
            content_type: None,
            parent_comment_id: None,
        };
        let ctx = TestContext::new();
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(AppError::ValidationError(_)) => {} // 期望
            _ => panic!("Expected ValidationError for content too long"),
        }
    }

    /// 测试无效颜色格式
    #[test]
    fn test_create_team_workflow_status_invalid_color() {
        let cmd = CreateTeamWorkflowStatusCommand {
            name: "Test".to_string(),
            category: "backlog".to_string(),
            color: "not-a-color".to_string(), // 无效颜色
            description: None,
            position: 0,
        };
        let ctx = TestContext::new();
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(AppError::ValidationError(_)) => {} // 期望
            _ => panic!("Expected ValidationError for invalid color"),
        }
    }

    /// 测试负数 position
    #[test]
    fn test_create_team_workflow_status_negative_position() {
        let cmd = CreateTeamWorkflowStatusCommand {
            name: "Test".to_string(),
            category: "backlog".to_string(),
            color: "#FF0000".to_string(),
            description: None,
            position: -1,
        };
        let ctx = TestContext::new();
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(AppError::ValidationError(_)) => {} // 期望
            _ => panic!("Expected ValidationError for negative position"),
        }
    }

    /// 测试特殊字符（Unicode/Emoji）
    #[test]
    fn test_create_team_workflow_status_unicode_name() {
        let cmd = CreateTeamWorkflowStatusCommand {
            name: "状态 🔥".to_string(), // 中文 + Emoji
            category: "backlog".to_string(),
            color: "#FF0000".to_string(),
            description: None,
            position: 0,
        };
        let ctx = TestContext::new();
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Response(_) => {} // Unicode 应该被支持
            HandleResult::Error(e) => panic!(
                "Unicode names should be supported but got error: {:?}", e
            ),
        }
    }
}
```

#### 2.3.3 权限测试

```rust
#[cfg(test)]
mod permission_tests {
    use super::*;

    /// 测试非 admin 不能更新 Workspace Member 角色
    #[test]
    fn test_update_workspace_member_permission_denied() {
        // Given: 以 member 身份连接
        let ctx = TestContext::with_role(Role::Member);
        let target_user_id = Uuid::new_v4();
        
        let cmd = UpdateWorkspaceMemberCommand {
            user_id: target_user_id,
            data: UpdateWorkspaceMemberData { role: Some("admin".to_string()) },
        };
        
        // When
        let result = block_on(handle_command(cmd, &ctx));
        
        // Then
        match result {
            HandleResult::Error(AppError::PermissionDenied) => {} // 期望
            _ => panic!("Expected PermissionDenied"),
        }
    }

    /// 测试非 admin 不能删除 Workspace Member
    #[test]
    fn test_delete_workspace_member_permission_denied() {
        let ctx = TestContext::with_role(Role::Member);
        let target_user_id = Uuid::new_v4();
        
        let cmd = DeleteWorkspaceMemberCommand {
            user_id: target_user_id,
        };
        
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(AppError::PermissionDenied) => {}
            _ => panic!("Expected PermissionDenied"),
        }
    }

    /// 测试 admin 可以删除 member
    #[test]
    fn test_admin_can_delete_workspace_member() {
        // Given: 以 admin 身份
        let ctx = TestContext::with_role(Role::Admin);
        let target_user_id = Uuid::new_v4();
        
        // 预先创建目标用户
        block_on(async {
            create_test_user(&ctx.pool, target_user_id).await;
        });
        
        let cmd = DeleteWorkspaceMemberCommand {
            user_id: target_user_id,
        };
        
        // When
        let result = block_on(handle_command(cmd, &ctx));
        
        // Then
        match result {
            HandleResult::Response(resp) => {
                assert!(resp.success);
            }
            _ => panic!("Admin should be able to delete member"),
        }
    }

    /// 测试非 admin 不能删除 Team
    #[test]
    fn test_delete_team_permission_denied() {
        let ctx = TestContext::with_role(Role::Member);
        let team_id = Uuid::new_v4();
        
        let cmd = DeleteTeamCommand { team_id };
        
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(AppError::PermissionDenied) => {}
            _ => panic!("Expected PermissionDenied for non-admin"),
        }
    }

    /// 测试非 owner 不能删除 Workspace
    #[test]
    fn test_delete_workspace_owner_only() {
        let ctx = TestContext::with_role(Role::Admin); // admin 但不是 owner
        let workspace_id = Uuid::new_v4();
        
        let cmd = DeleteWorkspaceCommand { workspace_id };
        
        let result = block_on(handle_command(cmd, &ctx));
        
        // 只有 owner 才能删除 workspace
        match result {
            HandleResult::Error(AppError::PermissionDenied) => {}
            HandleResult::Response(_) => panic!("Non-owner should not delete workspace"),
        }
    }
}
```

#### 2.3.4 安全测试

```rust
#[cfg(test)]
mod security_tests {
    use super::*;

    /// 测试 SQL 注入 - Team name
    #[test]
    fn test_create_team_sql_injection() {
        let malicious_input = "'; DROP TABLE teams; --";
        let cmd = CreateTeamCommand {
            name: malicious_input.to_string(),
            team_key: "TEST".to_string(),
            description: None,
            icon_url: None,
            is_private: false,
        };
        let ctx = TestContext::new();
        let result = block_on(handle_command(cmd, &ctx));
        
        // 应该被 sanitize 或拒绝，而不是执行
        match result {
            HandleResult::Error(_) => {} // 期望被拒绝
            HandleResult::Response(resp) => {
                // 如果成功，说明可能被注入了
                assert!(!resp.success || resp.error.contains("SQL"), 
                    "SQL injection should be caught");
            }
        }
    }

    /// 测试 SQL 注入 - Comment content
    #[test]
    fn test_create_comment_sql_injection() {
        let malicious_input = "'; DELETE FROM comments WHERE 1=1; --";
        let cmd = CreateCommentCommand {
            content: malicious_input.to_string(),
            content_type: None,
            parent_comment_id: None,
        };
        let ctx = TestContext::new();
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(_) => {} // 期望被拒绝
            HandleResult::Response(_) => panic!("SQL injection should be rejected"),
        }
    }

    /// 测试 XSS payload
    #[test]
    fn test_create_comment_xss_payload() {
        let xss_input = "<script>alert('xss')</script>";
        let cmd = CreateCommentCommand {
            content: xss_input.to_string(),
            content_type: None,
            parent_comment_id: None,
        };
        let ctx = TestContext::new();
        let result = block_on(handle_command(cmd, &ctx));
        
        // XSS payload 应该被转义或拒绝
        match result {
            HandleResult::Response(resp) => {
                // 如果成功，内容应该被转义
                if resp.success {
                    assert!(!resp.data["content"].as_str().unwrap().contains("<script>"),
                        "XSS should be escaped");
                }
            }
            HandleResult::Error(_) => {} // 被拒绝也是可以的
        }
    }

    /// 测试水平越权 - 用户不能访问其他 Workspace
    #[test]
    fn test_user_cannot_access_other_workspace() {
        // Given: 用户 A 属于 Workspace 1
        let ctx_a = TestContext::with_workspace(Role::Member, workspace_id_a);
        
        // 用户 A 尝试访问 Workspace 2
        let cmd = GetWorkspaceCommand {
            workspace_id: workspace_id_b, // 不属于用户 A 的 workspace
        };
        
        let result = block_on(handle_command(cmd, &ctx_a));
        
        match result {
            HandleResult::Error(AppError::PermissionDenied) => {} // 期望
            HandleResult::Error(AppError::NotFound) => {} // 期望（隐藏不存在）
            _ => panic!("Should not access other workspace"),
        }
    }

    /// 测试水平越权 - 用户不能访问其他 Team
    #[test]
    fn test_user_cannot_access_other_team() {
        let ctx = TestContext::with_workspace(Role::Member, workspace_a);
        let team_id = team_in_workspace_b; // 属于其他 workspace 的 team
        
        let cmd = GetTeamCommand { team_id };
        
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(AppError::PermissionDenied) => {}
            HandleResult::Error(AppError::NotFound) => {}
            _ => panic!("Should not access other team's data"),
        }
    }

    /// 测试垂直越权 - 成员不能提升自己为 admin
    #[test]
    fn test_member_cannot_promote_self_to_admin() {
        let ctx = TestContext::with_role(Role::Member);
        let self_user_id = ctx.user_id;
        
        let cmd = UpdateWorkspaceMemberCommand {
            user_id: self_user_id, // 尝试提升自己
            data: UpdateWorkspaceMemberData { role: Some("admin".to_string()) },
        };
        
        let result = block_on(handle_command(cmd, &ctx));
        
        match result {
            HandleResult::Error(AppError::PermissionDenied) => {}
            _ => panic!("Member should not be able to promote self to admin"),
        }
    }

    /// 测试请求伪造 - 验证 request_id 格式
    #[test]
    fn test_invalid_request_id_format() {
        let cmd = GetTeamCommand { team_id };
        
        // 使用无效的 request_id
        let result = block_on(handle_command_with_request_id(
            cmd, 
            "not-a-uuid".to_string(), // 无效 UUID
            &ctx
        ));
        
        match result {
            HandleResult::Error(AppError::ValidationError(msg)) => {
                assert!(msg.contains("request_id") || msg.contains("UUID"));
            }
            _ => panic!("Invalid request_id should be rejected"),
        }
    }
}
```

### 2.4 运行单元测试

```bash
# 运行所有 websocket 单元测试
cargo test --lib websocket_tests

# 运行特定测试模块
cargo test --lib websocket_tests::handler_tests
cargo test --lib websocket_tests::boundary_tests
cargo test --lib websocket_tests::permission_tests
cargo test --lib websocket_tests::security_tests

# 运行特定测试
cargo test --lib websocket_tests::test_team_commands_serialization

# 带日志输出
RUST_LOG=debug cargo test --lib websocket_tests -- --nocapture

# 运行所有单元测试（包括其他模块）
cargo test --lib
```

---

## 三、集成测试 (Integration Tests)

### 3.1 概述
- **位置**: `tests/websocket/command_integration_tests.rs`
- **运行**: `cargo test -- --ignored` (需要服务器)
- **特点**: 真实 WebSocket 连接 + 真实数据库

### 3.2 测试环境配置

#### 3.2.1 Docker Compose 配置

```yaml
# docker-compose.test.yml
version: "3.8"

services:
  postgres_test:
    image: postgres:15-alpine
    container_name: momentum-postgres-test
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: rust-backend-test
    ports:
      - "5435:5432"
    tmpfs:
      - /var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 3s
      retries: 3

  redis_test:
    image: redis:7-alpine
    container_name: momentum-redis-test
    ports:
      - "6380:6379"
    tmpfs:
      - /data

  backend_test:
    build: .
    container_name: momentum-backend-test
    environment:
      DATABASE_URL: postgres://postgres:postgres@postgres_test:5432/rust-backend-test
      REDIS_URL: redis://redis_test:6379/
      JWT_SECRET: test-secret-key
    ports:
      - "8001:8000"
    depends_on:
      postgres_test:
        condition: service_healthy
      redis_test:
        condition: service_healthy
```

#### 3.2.2 环境变量

```bash
# .env.test
DATABASE_URL=postgresql://postgres:postgres@localhost:5435/rust-backend-test
REDIS_URL=redis://localhost:6380/
JWT_SECRET=test-secret-key-for-integration-tests
WEBSOCKET_URL=ws://localhost:8001/ws
```

### 3.3 测试夹具 (Fixtures)

```rust
// tests/websocket/fixtures.rs

use diesel::{Connection, SqliteConnection};
use std::sync::Mutex;

pub struct TestFixture {
    pub workspace_id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub admin_user_id: Uuid,
    pub ws_connection: WebSocketConnection,
    pub db_pool: Pool,
    // 用于事务回滚的连接
    transaction: Mutex<Option<SqliteConnection>>,
}

impl TestFixture {
    pub async fn setup() -> Self {
        // 1. 创建数据库连接池
        let db_pool = create_test_pool().await;

        // 2. 创建测试工作空间
        let workspace_id = create_workspace(&db_pool, "Test Workspace").await;

        // 3. 创建测试用户
        let user_id = create_user(&db_pool, "test@example.com").await;
        let admin_user_id = create_admin_user(&db_pool, "admin@example.com").await;

        // 4. 创建测试团队
        let team_id = create_team(&db_pool, workspace_id, "Test Team").await;

        // 5. 添加用户到团队
        add_user_to_team(&db_pool, team_id, user_id, "member").await;
        add_user_to_team(&db_pool, team_id, admin_user_id, "admin").await;

        // 6. 建立 WebSocket 连接
        let ws_connection = create_ws_connection(workspace_id, user_id).await;

        Self {
            workspace_id,
            team_id,
            user_id,
            admin_user_id,
            ws_connection,
            db_pool,
            transaction: Mutex::new(None),
        }
    }

    /// 使用事务回滚方式保证测试隔离
    /// 所有操作在事务中进行，测试结束后自动回滚
    pub async fn with_transaction<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut SqliteConnection) -> T,
    {
        let mut conn = self.db_pool.get().await.unwrap();
        conn.begin_test_transaction().unwrap();
        
        let result = f(&mut conn);
        
        // 事务自动回滚，不需要手动 cleanup
        result
    }

    pub async fn teardown(&self) {
        // 使用事务回滚后，这里只需要关闭 WebSocket 连接
        self.ws_connection.close().await;
        
        // 如果使用手动 cleanup，需要清理所有关联数据
        cleanup_test_data(&self.db_pool, self.workspace_id).await;
    }
}

/// 完整的测试数据清理（备用方案）
async fn cleanup_test_data(pool: &Pool, workspace_id: Uuid) {
    let mut conn = pool.get().await.unwrap();
    
    // 按依赖顺序删除（先删除子表，再删主表）
    diesel::delete(
        schema::comments::table
            .filter(schema::comments::issue_id.eq_any(
                schema::issues::table
                    .filter(schema::issues::team_id.eq_any(
                        schema::teams::table
                            .filter(schema::teams::workspace_id.eq(workspace_id))
                            .select(schema::teams::id)
                    ))
                    .select(schema::issues::id)
            ))
    ).execute(&mut conn).await;
    
    diesel::delete(
        schema::issues::table
            .filter(schema::issues::team_id.eq_any(
                schema::teams::table
                    .filter(schema::teams::workspace_id.eq(workspace_id))
                    .select(schema::teams::id)
            ))
    ).execute(&mut conn).await;
    
    diesel::delete(
        schema::team_members::table
            .filter(schema::team_members::team_id.eq_any(
                schema::teams::table
                    .filter(schema::teams::workspace_id.eq(workspace_id))
                    .select(schema::teams::id)
            ))
    ).execute(&mut conn).await;
    
    diesel::delete(
        schema::teams::table
            .filter(schema::teams::workspace_id.eq(workspace_id))
    ).execute(&mut conn).await;
    
    diesel::delete(
        schema::workspace_members::table
            .filter(schema::workspace_members::workspace_id.eq(workspace_id))
    ).execute(&mut conn).await;
    
    diesel::delete(
        schema::workspaces::table
            .filter(schema::workspaces::id.eq(workspace_id))
    ).execute(&mut conn).await;
}
```

### 3.4 完整的 CRUD 流程测试

#### 3.4.1 Workspace CRUD 流程

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_workspace_full_crud_flow() {
    let fixture = TestFixture::setup().await;
    let admin_ws = create_ws_connection(fixture.workspace_id, fixture.admin_user_id).await;

    // ===== CREATE =====
    let create_response = send_command(&admin_ws, json!({
        "type": "create_workspace",
        "request_id": Uuid::new_v4().to_string(),
        "data": {
            "name": "New Workspace",
            "url_key": format!("test-ws-{}", Uuid::new_v4().to_string().replace("-", "").chars().take(8).collect::<String>())
        },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(create_response["success"].as_bool().unwrap_or(false));
    let workspace_id = create_response["data"]["id"].as_str().unwrap();

    // ===== READ =====
    let get_response = send_command(&admin_ws, json!({
        "type": "get_workspace",
        "request_id": Uuid::new_v4().to_string(),
        "workspace_id": workspace_id,
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(get_response["success"].as_bool().unwrap_or(false));
    assert_eq!(get_response["data"]["name"], "New Workspace");

    // ===== UPDATE =====
    let update_response = send_command(&admin_ws, json!({
        "type": "update_workspace",
        "request_id": Uuid::new_v4().to_string(),
        "workspace_id": workspace_id,
        "data": { "name": "Updated Workspace" },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(update_response["success"].as_bool().unwrap_or(false));
    assert_eq!(update_response["data"]["name"], "Updated Workspace");

    // ===== DELETE =====
    let delete_response = send_command(&admin_ws, json!({
        "type": "delete_workspace",
        "request_id": Uuid::new_v4().to_string(),
        "workspace_id": workspace_id,
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(delete_response["success"].as_bool().unwrap_or(false));

    fixture.teardown().await;
}
```

#### 3.4.2 Team CRUD 流程

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_team_full_crud_flow() {
    let fixture = TestFixture::setup().await;
    let admin_ws = create_ws_connection(fixture.workspace_id, fixture.admin_user_id).await;

    // ===== CREATE =====
    let create_response = send_command(&admin_ws, json!({
        "type": "create_team",
        "request_id": Uuid::new_v4().to_string(),
        "workspace_id": fixture.workspace_id.to_string(),
        "data": {
            "name": "New Team",
            "team_key": format!("NT-{}", Uuid::new_v4().to_string().replace("-", "").chars().take(4).collect::<String>().to_uppercase()),
            "description": "A new team for testing"
        },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(create_response["success"].as_bool().unwrap_or(false));
    let team_id = create_response["data"]["id"].as_str().unwrap();

    // ===== READ =====
    let get_response = send_command(&admin_ws, json!({
        "type": "get_team",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": team_id,
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(get_response["success"].as_bool().unwrap_or(false));
    assert_eq!(get_response["data"]["name"], "New Team");

    // ===== UPDATE =====
    let update_response = send_command(&admin_ws, json!({
        "type": "update_team",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": team_id,
        "data": { "name": "Updated Team Name" },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(update_response["success"].as_bool().unwrap_or(false));
    assert_eq!(update_response["data"]["name"], "Updated Team Name");

    // ===== DELETE =====
    let delete_response = send_command(&admin_ws, json!({
        "type": "delete_team",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": team_id,
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(delete_response["success"].as_bool().unwrap_or(false));

    fixture.teardown().await;
}
```

#### 3.4.3 Issue CRUD 流程

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_issue_full_crud_flow() {
    let fixture = TestFixture::setup().await;
    let admin_ws = create_ws_connection(fixture.workspace_id, fixture.admin_user_id).await;

    // 先创建一个 workflow status 作为前置条件
    let status_response = send_command(&admin_ws, json!({
        "type": "create_team_workflow_status",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": fixture.team_id.to_string(),
        "data": {
            "name": "To Do",
            "category": "backlog",
            "color": "#4A90E2",
            "position": 0
        },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;
    let status_id = status_response["data"]["id"].as_str().unwrap();

    // ===== CREATE =====
    let create_response = send_command(&admin_ws, json!({
        "type": "create_issue",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": fixture.team_id.to_string(),
        "data": {
            "title": "Test Issue",
            "description": "Issue description for testing",
            "status_id": status_id,
            "priority": "high"
        },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(create_response["success"].as_bool().unwrap_or(false));
    let issue_id = create_response["data"]["id"].as_str().unwrap();

    // ===== READ =====
    let get_response = send_command(&admin_ws, json!({
        "type": "get_issue",
        "request_id": Uuid::new_v4().to_string(),
        "issue_id": issue_id,
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(get_response["success"].as_bool().unwrap_or(false));
    assert_eq!(get_response["data"]["title"], "Test Issue");

    // ===== UPDATE =====
    let update_response = send_command(&admin_ws, json!({
        "type": "update_issue",
        "request_id": Uuid::new_v4().to_string(),
        "issue_id": issue_id,
        "data": { 
            "title": "Updated Issue Title",
            "status_id": status_id
        },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(update_response["success"].as_bool().unwrap_or(false));
    assert_eq!(update_response["data"]["title"], "Updated Issue Title");

    // ===== DELETE =====
    let delete_response = send_command(&admin_ws, json!({
        "type": "delete_issue",
        "request_id": Uuid::new_v4().to_string(),
        "issue_id": issue_id,
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(delete_response["success"].as_bool().unwrap_or(false));

    fixture.teardown().await;
}
```

#### 3.4.4 Comment CRUD 流程

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_comment_full_crud_flow() {
    // Setup
    let fixture = TestFixture::setup().await;
    let admin_ws = create_ws_connection(fixture.workspace_id, fixture.admin_user_id).await;
    
    // 创建 Issue 作为 Comment 的前置条件
    let status_response = send_command(&admin_ws, json!({
        "type": "create_team_workflow_status",
        "team_id": fixture.team_id.to_string(),
        "data": { "name": "To Do", "category": "backlog", "color": "#4A90E2", "position": 0 },
        ...
    })).await;
    let status_id = status_response["data"]["id"].as_str().unwrap();
    
    let issue_response = send_command(&admin_ws, json!({
        "type": "create_issue",
        "team_id": fixture.team_id.to_string(),
        "data": { "title": "Test Issue", "status_id": status_id },
        ...
    })).await;
    let issue_id = issue_response["data"]["id"].as_str().unwrap();

    // ===== CREATE =====
    let create_response = send_command(&fixture.ws_connection, json!({
        "type": "create_comment",
        "request_id": Uuid::new_v4().to_string(),
        "issue_id": issue_id.to_string(),
        "data": {
            "content": "Initial comment",
            "content_type": "markdown"
        },
        "meta": {
            "workspaceId": fixture.workspace_id.to_string(),
            "userId": fixture.user_id.to_string()
        }
    })).await;

    assert!(create_response["success"].as_bool().unwrap_or(false));
    let comment_id = create_response["data"]["id"].as_str().unwrap();

    // ===== READ =====
    let query_response = send_command(&fixture.ws_connection, json!({
        "type": "query_comments",
        "request_id": Uuid::new_v4().to_string(),
        "issue_id": issue_id.to_string(),
        "meta": {
            "workspaceId": fixture.workspace_id.to_string(),
            "userId": fixture.user_id.to_string()
        }
    })).await;

    let comments = &query_response["data"].as_array().unwrap();
    assert!(comments.iter().any(|c| c["id"] == comment_id));

    // ===== UPDATE =====
    let update_response = send_command(&fixture.ws_connection, json!({
        "type": "update_comment",
        "request_id": Uuid::new_v4().to_string(),
        "issue_id": issue_id.to_string(),
        "comment_id": comment_id,
        "data": { "content": "Updated comment content" },
        "meta": {
            "workspaceId": fixture.workspace_id.to_string(),
            "userId": fixture.user_id.to_string()
        }
    })).await;

    assert_eq!(
        update_response["data"]["content"].as_str().unwrap(),
        "Updated comment content"
    );

    // ===== DELETE =====
    let delete_response = send_command(&fixture.ws_connection, json!({
        "type": "delete_comment",
        "request_id": Uuid::new_v4().to_string(),
        "issue_id": issue_id.to_string(),
        "comment_id": comment_id,
        "meta": {
            "workspaceId": fixture.workspace_id.to_string(),
            "userId": fixture.user_id.to_string()
        }
    })).await;

    assert!(delete_response["success"].as_bool().unwrap_or(false));

    fixture.teardown().await;
}
```

#### 3.4.5 Team Workflow Status CRUD 流程

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_team_workflow_status_full_crud_flow() {
    let fixture = TestFixture::setup().await;

    // ===== CREATE =====
    let create_response = send_command(&fixture.ws_connection, json!({
        "type": "create_team_workflow_status",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": fixture.team_id.to_string(),
        "data": {
            "name": "In Progress",
            "description": "Issues being worked on",
            "color": "#FF6B6B",
            "category": "started",
            "position": 0
        },
        "meta": {
            "workspaceId": fixture.workspace_id.to_string(),
            "userId": fixture.user_id.to_string()
        }
    })).await;

    let status_id = create_response["data"]["id"].as_str().unwrap();

    // ===== READ =====
    let get_response = send_command(&fixture.ws_connection, json!({
        "type": "get_team_workflow_statuses",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": fixture.team_id.to_string(),
        "meta": {
            "workspaceId": fixture.workspace_id.to_string(),
            "userId": fixture.user_id.to_string()
        }
    })).await;

    let statuses = &get_response["data"].as_array().unwrap();
    assert!(statuses.iter().any(|s| s["id"] == status_id));

    // ===== UPDATE =====
    let update_response = send_command(&fixture.ws_connection, json!({
        "type": "update_team_workflow_status",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": fixture.team_id.to_string(),
        "status_id": status_id,
        "data": { "name": "In Development", "color": "#4ECDC4" },
        "meta": {
            "workspaceId": fixture.workspace_id.to_string(),
            "userId": fixture.user_id.to_string()
        }
    })).await;

    assert_eq!(
        update_response["data"]["name"].as_str().unwrap(),
        "In Development"
    );

    // ===== DELETE =====
    let delete_response = send_command(&fixture.ws_connection, json!({
        "type": "delete_team_workflow_status",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": fixture.team_id.to_string(),
        "status_id": status_id,
        "meta": {
            "workspaceId": fixture.workspace_id.to_string(),
            "userId": fixture.user_id.to_string()
        }
    })).await;

    assert!(delete_response["success"].as_bool().unwrap_or(false));

    fixture.teardown().await;
}
```

#### 3.4.6 Workspace Member CRUD 流程

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_workspace_member_crud_flow() {
    let fixture = TestFixture::setup().await;
    let new_user_id = create_test_user(&fixture.db_pool, "newuser@example.com").await;
    let admin_ws = create_ws_connection(fixture.workspace_id, fixture.admin_user_id).await;

    // ===== CREATE (Invite) =====
    let invite_response = send_command(&admin_ws, json!({
        "type": "create_workspace_invitation",
        "request_id": Uuid::new_v4().to_string(),
        "workspace_id": fixture.workspace_id.to_string(),
        "data": {
            "email": "newuser@example.com",
            "role": "member"
        },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;
    
    assert!(invite_response["success"].as_bool().unwrap_or(false));
    let invitation_id = invite_response["data"]["id"].as_str().unwrap();

    // ===== ACCEPT INVITATION (As new user) =====
    let new_user_ws = create_ws_connection(fixture.workspace_id, new_user_id).await;
    let accept_response = send_command(&new_user_ws, json!({
        "type": "accept_workspace_invitation",
        "request_id": Uuid::new_v4().to_string(),
        "invitation_id": invitation_id,
        "meta": {
            "userId": new_user_id.to_string()
        }
    })).await;
    
    assert!(accept_response["success"].as_bool().unwrap_or(false));

    // ===== UPDATE (Role) =====
    let update_response = send_command(&admin_ws, json!({
        "type": "update_workspace_member",
        "request_id": Uuid::new_v4().to_string(),
        "user_id": new_user_id.to_string(),
        "data": { "role": "admin" },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert_eq!(
        update_response["data"]["role"].as_str().unwrap(),
        "admin"
    );

    // ===== READ =====
    let get_response = send_command(&admin_ws, json!({
        "type": "get_workspace_member",
        "request_id": Uuid::new_v4().to_string(),
        "user_id": new_user_id.to_string(),
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(get_response["success"].as_bool().unwrap_or(false));

    // ===== DELETE =====
    let delete_response = send_command(&admin_ws, json!({
        "type": "delete_workspace_member",
        "request_id": Uuid::new_v4().to_string(),
        "user_id": new_user_id.to_string(),
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;

    assert!(delete_response["success"].as_bool().unwrap_or(false));
}
```

#### 3.4.7 Team Member 管理流程

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_team_member_management_flow() {
    let fixture = TestFixture::setup().await;
    let admin_ws = create_ws_connection(fixture.workspace_id, fixture.admin_user_id).await;
    let new_user_id = create_test_user(&fixture.db_pool, "teammate@example.com").await;

    // ===== ADD MEMBER =====
    let add_response = send_command(&admin_ws, json!({
        "type": "add_team_member",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": fixture.team_id.to_string(),
        "data": {
            "user_id": new_user_id.to_string(),
            "role": "developer"
        },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;
    
    assert!(add_response["success"].as_bool().unwrap_or(false));

    // ===== UPDATE MEMBER ROLE =====
    let update_response = send_command(&admin_ws, json!({
        "type": "update_team_member",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": fixture.team_id.to_string(),
        "user_id": new_user_id.to_string(),
        "data": { "role": "admin" },
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;
    
    assert_eq!(update_response["data"]["role"], "admin");

    // ===== REMOVE MEMBER =====
    let remove_response = send_command(&admin_ws, json!({
        "type": "remove_team_member",
        "request_id": Uuid::new_v4().to_string(),
        "team_id": fixture.team_id.to_string(),
        "user_id": new_user_id.to_string(),
        "meta": {
            "userId": fixture.admin_user_id.to_string()
        }
    })).await;
    
    assert!(remove_response["success"].as_bool().unwrap_or(false));
}
```

### 3.5 并发测试

#### 3.5.1 并发评论编辑

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_concurrent_comment_edits() {
    let fixture = TestFixture::setup().await;
    let admin_ws = create_ws_connection(fixture.workspace_id, fixture.admin_user_id).await;
    
    // 创建 Issue
    let status_response = send_command(&admin_ws, json!({
        "type": "create_team_workflow_status",
        "team_id": fixture.team_id.to_string(),
        "data": { "name": "To Do", "category": "backlog", "color": "#4A90E2", "position": 0 },
        ...
    })).await;
    let status_id = status_response["data"]["id"].as_str().unwrap();
    
    let issue_response = send_command(&admin_ws, json!({
        "type": "create_issue",
        "team_id": fixture.team_id.to_string(),
        "data": { "title": "Concurrent Test Issue", "status_id": status_id },
        ...
    })).await;
    let issue_id = issue_response["data"]["id"].as_str().unwrap();
    
    // 创建评论
    let comment_response = send_command(&admin_ws, json!({
        "type": "create_comment",
        "issue_id": issue_id,
        "data": { "content": "Original content" },
        ...
    })).await;
    let comment_id = comment_response["data"]["id"].as_str().unwrap();
    
    // 并发更新同一评论
    let handles: Vec<_> = (0..5).map(|i| {
        let ws = create_ws_connection(fixture.workspace_id, fixture.admin_user_id).await;
        let issue_id = issue_id.to_string();
        let comment_id = comment_id.to_string();
        
        tokio::spawn(async move {
            send_command(&ws, json!({
                "type": "update_comment",
                "issue_id": issue_id,
                "comment_id": comment_id,
                "data": { "content": format!("Update {}", i) }
            })).await
        })
    }).collect();
    
    let results = futures::future::join_all(handles).await;
    
    // 验证所有请求都得到响应（可能有部分失败，这是预期行为）
    for result in results {
        assert!(result.is_ok(), "Request should complete");
    }
    
    // 验证最终状态一致性 - 获取最新评论内容
    let final_response = send_command(&admin_ws, json!({
        "type": "query_comments",
        "issue_id": issue_id,
        ...
    })).await;
    
    let final_comment = final_response["data"].as_array().unwrap()
        .iter().find(|c| c["id"] == comment_id);
    
    assert!(final_comment.is_some());
    // 最终内容应该是其中一次更新
    let final_content = final_comment.unwrap()["content"].as_str().unwrap();
    assert!(final_content.starts_with("Update "), 
        "Content should be one of the updates, got: {}", final_content);
}
```

#### 3.5.2 并发团队创建

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_concurrent_team_creation() {
    let fixture = TestFixture::setup().await;
    let admin_ws = create_ws_connection(fixture.workspace_id, fixture.admin_user_id).await;
    
    // 并发创建多个 team，使用不同的 team_key
    let handles: Vec<_> = (0..10).map(|i| {
        let ws = admin_ws.clone();
        let workspace_id = fixture.workspace_id.to_string();
        
        tokio::spawn(async move {
            send_command(&ws, json!({
                "type": "create_team",
                "workspace_id": workspace_id,
                "data": {
                    "name": format!("Team {}", i),
                    "team_key": format!("TST-CONC-{}", i),
                    "description": format!("Concurrent team {}", i)
                }
            })).await
        })
    }).collect();
    
    let results = futures::future::join_all(handles).await;
    
    // 验证所有创建都成功
    let success_count = results.iter()
        .filter(|r| r.as_ref().unwrap()["success"].as_bool().unwrap_or(false))
        .count();
    
    assert_eq!(success_count, 10, "All concurrent team creations should succeed");
    
    // 验证没有重复的 team_key
    let query_response = send_command(&admin_ws, json!({
        "type": "query_teams",
        "workspace_id": fixture.workspace_id.to_string(),
        ...
    })).await;
    
    let teams = query_response["data"].as_array().unwrap();
    let team_keys: Vec<&str> = teams.iter()
        .filter_map(|t| t["team_key"].as_str())
        .collect();
    
    let unique_keys: HashSet<&str> = team_keys.iter().cloned().collect();
    assert_eq!(team_keys.len(), unique_keys.len(), "Team keys should be unique");
}
```

### 3.6 WebSocket 协议测试

#### 3.6.1 重连测试

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_websocket_reconnection() {
    let fixture = TestFixture::setup().await;
    
    // 第一次连接并执行操作
    let response1 = send_command(&fixture.ws_connection, json!({
        "type": "get_team",
        "team_id": fixture.team_id.to_string(),
        ...
    })).await;
    
    assert!(response1["success"].as_bool().unwrap_or(false));
    
    // 断开连接
    fixture.ws_connection.close().await;
    
    // 重新连接
    let new_ws = create_ws_connection(fixture.workspace_id, fixture.user_id).await;
    
    // 验证可以继续操作
    let response2 = send_command(&new_ws, json!({
        "type": "get_team",
        "team_id": fixture.team_id.to_string(),
        ...
    })).await;
    
    assert!(response2["success"].as_bool().unwrap_or(false));
    
    new_ws.close().await;
}
```

#### 3.6.2 心跳测试

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_websocket_heartbeat() {
    let fixture = TestFixture::setup().await;
    
    // 等待心跳间隔
    tokio::time::sleep(Duration::from_secs(35)).await;
    
    // 发送命令验证连接仍然有效
    let response = send_command(&fixture.ws_connection, json!({
        "type": "get_team",
        "team_id": fixture.team_id.to_string(),
        ...
    })).await;
    
    // 如果心跳正常工作，连接应该保持
    assert!(response["success"].as_bool().unwrap_or(false) 
        || response["error"].as_str().unwrap_or("").contains("timeout"),
        "Connection should be alive or timeout properly");
}
```

#### 3.6.3 无效消息测试

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_websocket_invalid_json() {
    let fixture = TestFixture::setup().await;
    
    // 发送无效 JSON
    let result = send_raw_message(&fixture.ws_connection, "{ invalid json }").await;
    
    // 应该收到错误响应
    assert!(result.is_err() || result.as_ref().unwrap()["error"].is_string());
}
```

#### 3.6.4 消息顺序测试

```rust
#[tokio::test]
#[ignore = "requires test environment"]
async fn test_websocket_message_ordering() {
    let fixture = TestFixture::setup().await;
    
    // 发送多个命令
    let request_ids: Vec<String> = (0..5).map(|_| Uuid::new_v4().to_string()).collect();
    
    for (i, request_id) in request_ids.iter().enumerate() {
        let response = send_command(&fixture.ws_connection, json!({
            "type": "get_team",
            "request_id": request_id,
            "team_id": fixture.team_id.to_string(),
            ...
        })).await;
        
        assert!(response["success"].as_bool().unwrap_or(false));
    }
    
    // 验证所有响应都能匹配到对应的 request_id
    // （实际实现可能使用异步响应队列）
}
```

### 3.7 测试矩阵

#### 3.7.1 完整命令覆盖矩阵

| 命令 | Create | Read | Update | Delete | 错误处理 | 并发测试 | 安全测试 |
|------|--------|-------|--------|--------|---------|---------|---------|
| `get_team` | - | ✅ | - | - | ✅ | ✅ | ✅ |
| `create_team` | ✅ | - | - | - | ✅ | ✅ | ✅ |
| `update_team` | - | ✅ | ✅ | - | ✅ | - | ✅ |
| `delete_team` | - | - | - | ✅ | ✅ | - | ✅ |
| `query_teams` | - | ✅ | - | - | ✅ | - | - |
| `get_team_workflow_statuses` | - | ✅ | - | - | ✅ | ✅ | - |
| `create_team_workflow_status` | ✅ | - | - | - | ✅ | ✅ | ✅ |
| `update_team_workflow_status` | - | ✅ | ✅ | - | ✅ | ✅ | ✅ |
| `delete_team_workflow_status` | - | - | - | ✅ | ✅ | - | - |
| `get_workspace` | - | ✅ | - | - | ✅ | ✅ | ✅ |
| `create_workspace` | ✅ | - | - | - | ✅ | ✅ | ✅ |
| `update_workspace` | - | ✅ | ✅ | - | ✅ | - | ✅ |
| `delete_workspace` | - | - | - | ✅ | ✅ | - | ✅ |
| `query_workspaces` | - | ✅ | - | - | ✅ | - | - |
| `get_workspace_member` | - | ✅ | - | - | ✅ | - | ✅ |
| `create_workspace_invitation` | ✅ | - | - | - | ✅ | - | ✅ |
| `accept_workspace_invitation` | ✅ | - | - | - | ✅ | - | ✅ |
| `update_workspace_member` | - | ✅ | ✅ | - | ✅ | ✅ | ✅ |
| `delete_workspace_member` | - | - | - | ✅ | ✅ | - | ✅ |
| `get_issue` | - | ✅ | - | - | ✅ | - | ✅ |
| `create_issue` | ✅ | - | - | - | ✅ | ✅ | ✅ |
| `update_issue` | - | ✅ | ✅ | - | ✅ | ✅ | ✅ |
| `delete_issue` | - | - | - | ✅ | ✅ | - | ✅ |
| `query_issues` | - | ✅ | - | - | ✅ | - | - |
| `query_comments` | - | ✅ | - | - | ✅ | ✅ | - |
| `create_comment` | ✅ | - | - | - | ✅ | ✅ | ✅ |
| `update_comment` | - | ✅ | ✅ | - | ✅ | ✅ | ✅ |
| `delete_comment` | - | - | - | ✅ | ✅ | - | - |
| `add_team_member` | ✅ | - | - | - | ✅ | ✅ | ✅ |
| `update_team_member` | - | ✅ | ✅ | - | ✅ | ✅ | ✅ |
| `remove_team_member` | - | - | - | ✅ | ✅ | - | ✅ |

#### 3.7.2 测试类型覆盖矩阵

| 测试类型 | 单元测试 | 集成测试 | 端到端测试 |
|---------|---------|---------|-----------|
| Happy Path | ✅ | ✅ | ✅ |
| 参数验证 | ✅ | ✅ | ✅ |
| 权限检查 | ✅ | ✅ | ✅ |
| 越权访问 | ✅ | ✅ | - |
| SQL 注入 | ✅ | - | - |
| XSS | ✅ | - | - |
| 并发修改 | - | ✅ | - |
| 重连机制 | - | ✅ | - |
| 事务回滚 | - | ✅ | - |
| 性能基准 | - | ✅ | ✅ |

### 3.8 运行集成测试

```bash
# 1. 启动测试环境
docker-compose -f docker-compose.test.yml up -d

# 2. 等待服务就绪
until pg_isready -h localhost -p 5435 -U postgres; do sleep 1; done
until redis-cli -h localhost -p 6380 ping | grep -q PONG; do sleep 1; done

# 3. 运行迁移
DATABASE_URL=postgresql://postgres:postgres@localhost:5435/rust-backend-test \
  diesel migration run

# 4. 运行集成测试（串行）
cargo test -- --ignored --test-threads=1

# 5. 运行集成测试（并行，可选）
cargo test -- --ignored --test-threads=4

# 6. 运行特定测试
cargo test -- --ignored test_comment_full_crud_flow --test-threads=1

# 7. 带日志运行
RUST_LOG=debug cargo test -- --ignored --test-threads=1 --nocapture

# 8. 清理
docker-compose -f docker-compose.test.yml down -v
```

---

## 四、性能/负载测试

### 4.1 性能测试目标

| 指标 | 目标值 | 警告阈值 |
|-----|-------|---------|
| WebSocket 连接建立时间 | < 100ms | > 200ms |
| 命令响应时间 (p95) | < 200ms | > 500ms |
| 命令响应时间 (p99) | < 500ms | > 1000ms |
| 并发连接数 | 100+ | 50 |
| 消息吞吐量 | 1000 msg/s | 500 msg/s |

### 4.2 k6 负载测试配置

```yaml
# .github/workflows/performance-test.yml

name: Performance Tests

on:
  schedule:
    - cron: '0 2 * * *'  # 每天凌晨运行
  workflow_dispatch:  # 手动触发

jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - name: Start test environment
        run: docker-compose -f docker-compose.test.yml up -d
      
      - name: Wait for services
        run: |
          until pg_isready -h localhost -p 5435 -U postgres; do sleep 1; done
          until redis-cli -h localhost -p 6380 ping | grep -q PONG; do sleep 1; done
      
      - name: Run k6 load test
        uses: grafana/k6-action@v0.2.0
        with:
          filename: tests/performance/websocket-load-test.js
          flags: '--out json=results.json'
      
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: k6-results
          path: results.json
```

### 4.3 k6 测试脚本

```javascript
// tests/performance/websocket-load-test.js

import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const errorRate = new Rate('errors');
const connectionTime = new Trend('connection_time');
const commandLatency = new Trend('command_latency');

export const options = {
  stages: [
    { duration: '30s', target: 10 },   // 预热
    { duration: '1m', target: 50 },    // 正常负载
    { duration: '30s', target: 100 },  // 峰值负载
    { duration: '1m', target: 100 },   // 持续峰值
    { duration: '30s', target: 0 },    // 冷却
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500'], 
    'connection_time': ['p(95)<200'],
    'errors': ['rate<0.05'],
  },
};

const BASE_URL = __ENV.WEBSOCKET_URL || 'ws://localhost:8001/ws';

export default function() {
  const teamId = `test-team-${Math.random().toString(36).substring(7)}`;
  const userId = `test-user-${Math.random().toString(36).substring(7)}`;
  
  // 测试连接建立时间
  const connectStart = Date.now();
  
  ws.connect(BASE_URL, {}, function(socket) {
    connectionTime.add(Date.now() - connectStart);
    
    socket.on('open', () => {
      // 模拟用户登录/认证（实际项目中可能需要）
      
      // 发送 GetTeam 命令
      const cmdStart = Date.now();
      socket.send(JSON.stringify({
        type: 'get_team',
        team_id: teamId,
        request_id: Math.random().toString(36).substring(7),
      }));
      
      socket.on('message', (data) => {
        commandLatency.add(Date.now() - cmdStart);
        
        const response = JSON.parse(data);
        if (!response.success) {
          errorRate.add(1);
        }
      });
      
      socket.on('error', (e) => {
        errorRate.add(1);
        console.error('WebSocket error:', e);
      });
      
      // 保持连接一段时间
      sleep(2);
      
      socket.close();
    });
  });
}
```

---

## 五、测试数据管理

### 5.1 测试数据工厂

```rust
// tests/factories/mod.rs

use diesel::{Connection, PgConnection};
use rand::Rng;
use std::sync::Mutex;

pub struct TestDataFactory {
    pool: Pool,
}

impl TestDataFactory {
    pub async fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// 使用事务自动回滚，保证测试隔离
    pub async fn with_transaction<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut PgConnection) -> T,
    {
        let mut conn = self.pool.get().await.unwrap();
        conn.begin_test_transaction().unwrap();
        
        let result = f(&mut conn);
        
        // 事务自动回滚，不需要手动 cleanup
        result
    }

    pub async fn workspace(&mut self, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        let url_key = format!("test-{}-{}", 
            id.to_string().replace("-", "").chars().take(8).collect::<String>(),
            rand::thread_rng().gen_range(0..9999)
        );
        
        diesel::insert_into(schema::workspaces::table)
            .values((
                schema::workspaces::id.eq(id),
                schema::workspaces::name.eq(name),
                schema::workspaces::url_key.eq(url_key),
                schema::workspaces::created_at.eq(Utc::now()),
                schema::workspaces::updated_at.eq(Utc::now()),
            ))
            .execute(&mut self.pool.get().await.unwrap())
            .unwrap();
        
        id
    }

    pub async fn team(&mut self, workspace_id: Uuid, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        let team_key = random_team_key();
        
        diesel::insert_into(schema::teams::table)
            .values((
                schema::teams::id.eq(id),
                schema::teams::workspace_id.eq(workspace_id),
                schema::teams::name.eq(name),
                schema::teams::team_key.eq(team_key),
                schema::teams::created_at.eq(Utc::now()),
                schema::teams::updated_at.eq(Utc::now()),
            ))
            .execute(&mut self.pool.get().await.unwrap())
            .unwrap();
        
        id
    }
    
    pub async fn issue(&mut self, team_id: Uuid, title: &str, status_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        
        diesel::insert_into(schema::issues::table)
            .values((
                schema::issues::id.eq(id),
                schema::issues::team_id.eq(team_id),
                schema::issues::title.eq(title),
                schema::issues::status_id.eq(status_id),
                schema::issues::created_at.eq(Utc::now()),
                schema::issues::updated_at.eq(Utc::now()),
            ))
            .execute(&mut self.pool.get().await.unwrap())
            .unwrap();
        
        id
    }
    
    pub async fn comment(&mut self, issue_id: Uuid, content: &str) -> Uuid {
        let id = Uuid::new_v4();
        
        diesel::insert_into(schema::comments::table)
            .values((
                schema::comments::id.eq(id),
                schema::comments::issue_id.eq(issue_id),
                schema::comments::content.eq(content),
                schema::comments::created_at.eq(Utc::now()),
                schema::comments::updated_at.eq(Utc::now()),
            ))
            .execute(&mut self.pool.get().await.unwrap())
            .unwrap();
        
        id
    }

    /// 清理测试数据（备用方案，不推荐使用）
    pub async fn cleanup(&self, workspace_id: Uuid) {
        let mut conn = self.pool.get().await.unwrap();
        
        // 按依赖顺序删除
        diesel::delete(
            schema::comments::table
                .filter(schema::comments::issue_id.eq_any(
                    schema::issues::table
                        .filter(schema::issues::team_id.eq_any(
                            schema::teams::table
                                .filter(schema::teams::workspace_id.eq(workspace_id))
                                .select(schema::teams::id)
                        ))
                        .select(schema::issues::id)
                ))
        ).execute(&mut conn).await.ok();
        
        diesel::delete(
            schema::issues::table
                .filter(schema::issues::team_id.eq_any(
                    schema::teams::table
                        .filter(schema::teams::workspace_id.eq(workspace_id))
                        .select(schema::teams::id)
                ))
        ).execute(&mut conn).await.ok();
        
        diesel::delete(
            schema::team_members::table
                .filter(schema::team_members::team_id.eq_any(
                    schema::teams::table
                        .filter(schema::teams::workspace_id.eq(workspace_id))
                        .select(schema::teams::id)
                ))
        ).execute(&mut conn).await.ok();
        
        diesel::delete(
            schema::teams::table
                .filter(schema::teams::workspace_id.eq(workspace_id))
        ).execute(&mut conn).await.ok();
        
        diesel::delete(
            schema::workspace_members::table
                .filter(schema::workspace_members::workspace_id.eq(workspace_id))
        ).execute(&mut conn).await.ok();
        
        diesel::delete(
            schema::workspaces::table
                .filter(schema::workspaces::id.eq(workspace_id))
        ).execute(&mut conn).await.ok();
    }
}
```

### 5.2 随机测试数据生成

```rust
// tests/utils/random.rs

use rand::Rng;

/// 生成随机团队 Key
pub fn random_team_key() -> String {
    let mut rng = rand::thread_rng();
    let random: String = (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 26 {
                (b'a' + idx) as char
            } else {
                (b'0' + idx - 26) as char
            }
        })
        .collect();
    format!("TST-{}", random.to_uppercase())
}

/// 生成随机邮箱
pub fn random_email() -> String {
    let team_key = random_team_key().to_lowercase();
    format!("test_{}@example.com", team_key)
}

/// 生成随机 URL Key
pub fn random_url_key() -> String {
    let mut rng = rand::thread_rng();
    let random: String = (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 26 {
                (b'a' + idx) as char
            } else {
                (b'0' + idx - 26) as char
            }
        })
        .collect();
    random
}

/// 生成随机颜色（十六进制）
pub fn random_color() -> String {
    let mut rng = rand::thread_rng();
    format!("#{:06x}", rng.gen_range(0..0xFFFFFF))
}

/// 生成随机长文本
pub fn random_long_text(max_length: usize) -> String {
    let mut rng = rand::thread_rng();
    let length = rng.gen_range(100..max_length);
    (0..length).map(|_| rng.gen::<char>()).collect()
}
```

---

## 六、CI/CD 集成

### 6.1 GitHub Actions Workflow（完整版）

```yaml
# .github/workflows/test.yml

name: Backend Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          components: clippy, rustfmt
      
      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Run Clippy
        run: cargo clippy -- -D warnings
      
      - name: Run unit tests
        run: cargo test --lib -- -q --test-threads=4
      
      - name: Run doctests
        run: cargo test --doc

  integration-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: rust-backend-test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
          -p 5435:5432
        ports:
          - 5435:5432
      
      redis:
        image: redis:7-alpine
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
          -p 6380:6379
        ports:
          - 6380:6379
    
    env:
      DATABASE_URL: postgresql://postgres:postgres@localhost:5435/rust-backend-test
      REDIS_URL: redis://localhost:6380/
    
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      
      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-integration-target-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Wait for PostgreSQL
        run: |
          until pg_isready -h localhost -p 5435 -U postgres; do sleep 1; done
      
      - name: Wait for Redis
        run: |
          until redis-cli -h localhost -p 6380 ping | grep -q PONG; do sleep 1; done
      
      - name: Run migrations
        run: diesel migration run --database-url=$DATABASE_URL
      
      - name: Run integration tests (serial)
        run: cargo test -- --ignored --test-threads=1
      
      - name: Run integration tests (parallel, safe ones)
        run: cargo test -- --ignored --test-threads=4
        continue-on-error: true

  security-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Run Cargo audit
        uses: rustsec/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
      
      - name: Run Cargo deny
        uses: rustsec/audit-check@v1
        with:
          command: 'deny check'
          token: ${{ secrets.GITHUB_TOKEN }}

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          components: llvm-tools-preview
      
      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov
      
      - name: Generate coverage report
        run: cargo llvm-cov --lib --lcov --output-path lcov.info
      
      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
          fail_ci_if_error: true
```

---

## 七、测试覆盖目标

### 7.1 覆盖率目标

| 测试类型 | 行覆盖率目标 | 分支覆盖率目标 | 说明 |
|---------|------------|--------------|-----|
| 单元测试 | 90%+ | 80%+ | Rust 标准是 90%+ |
| 集成测试 | 95%+ | 85%+ | 关键路径必须覆盖 |
| 安全测试 | 100% | 100% | 所有安全相关代码 |

### 7.2 增量覆盖率要求

```yaml
# PR 必须满足：
# - 总体覆盖率不得低于主分支 5%
# - 新增代码覆盖率必须 >= 80%
# - 安全相关代码覆盖率必须 100%
```

### 7.3 关键测试场景

1. **Comment 完整流程**: 创建 → 查看 → 编辑 → 删除
2. **Team Workflow 管理**: 创建状态 → 编辑 → 删除 → 重新排序
3. **Workspace Member 管理**: 邀请 → 接受 → 更新角色 → 移除
4. **Issue 完整流程**: 创建 → 查看 → 编辑 → 删除
5. **权限验证**: 非管理员操作应被拒绝
6. **越权访问**: 用户不能访问不属于自己的资源
7. **并发修改**: 多个请求同时修改同一资源
8. **安全注入**: SQL 注入、XSS 等攻击被阻止

---

## 八、待办事项

### 高优先级

- [ ] 创建 `docker-compose.test.yml`
- [ ] 实现 `TestFixture` 夹具（使用事务回滚）
- [ ] 完成 Workspace CRUD 集成测试
- [ ] 完成 Team CRUD 集成测试
- [ ] 完成 Issue CRUD 集成测试
- [ ] 完成 Comment CRUD 集成测试
- [ ] 完成 Team Workflow Status CRUD 集成测试
- [ ] 完成 Workspace Member CRUD 集成测试
- [ ] 完成 Team Member 管理集成测试
- [ ] 补充 Handler 逻辑单元测试
- [ ] 补充边界条件单元测试
- [ ] 配置 GitHub Actions CI/CD

### 中优先级

- [ ] 实现完整的测试数据工厂
- [ ] 添加权限测试（单元测试）
- [ ] 添加安全测试（SQL 注入、XSS）
- [ ] 添加并发测试
- [ ] 添加 WebSocket 协议测试
- [ ] 添加 Invitation 流程测试
- [ ] 添加性能/负载测试配置

### 低优先级（计划中）

- [ ] 集成 Cargo audit 安全审计
- [ ] 集成代码覆盖率报告
- [ ] 添加性能基准测试
- [ ] 添加混沌测试

---

## 九、相关文档

- [前端测试方案](../momentum-frontend/docs/frontend-testing-plan.md)
- [WebSocket 迁移计划](./websocket-migration-plan.md)
- [后端架构文档](./ARCHITECTURE.md)
- [安全编码规范](./SECURITY.md)
