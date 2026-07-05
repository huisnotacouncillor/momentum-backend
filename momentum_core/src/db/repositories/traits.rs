//! Repository trait 抽象
//!
//! P2.6 修复：定义 Repository trait，使服务层可以依赖抽象而非具体实现
//! 支持单元测试中 mock Repository 而无需启动真实数据库
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use crate::db::repositories::traits::IssueRepository;
//!
//! pub struct IssuesService<R: IssueRepository> {
//!     repo: R,
//! }
//!
//! impl<R: IssueRepository> IssuesService<R> {
//!     pub async fn create(&self, ...) -> Result<Issue, AppError> {
//!         self.repo.create(...).await
//!     }
//! }
//! ```

use async_trait::async_trait;
use diesel::PgConnection;
use uuid::Uuid;

use crate::db::models::issue::{Issue, IssueCursor, NewIssue};
use crate::error::AppError;

/// Issue 仓储 trait 抽象
#[async_trait]
pub trait IssueRepositoryTrait: Send + Sync {
    /// 按 ID 查找（强制工作区隔离）
    async fn find_by_id_in_workspace(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
        issue_id: Uuid,
    ) -> Result<Option<Issue>, AppError>;

    /// 列出工作区内的 Issues
    async fn list_by_workspace(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
    ) -> Result<Vec<Issue>, AppError>;

    /// 按 team 列出（支持分页）
    async fn list_by_team(
        &self,
        conn: &mut PgConnection,
        team_id: Uuid,
        limit: i64,
        cursor: Option<IssueCursor>,
    ) -> Result<Vec<Issue>, AppError>;

    /// 高级过滤查询
    async fn list_by_team_filtered(
        &self,
        conn: &mut PgConnection,
        team_id: Option<Uuid>,
        project_id: Option<Uuid>,
        assignee_id: Option<Uuid>,
        priority: Option<String>,
        search: Option<String>,
        limit: i64,
        cursor: Option<IssueCursor>,
    ) -> Result<Vec<Issue>, AppError>;

    /// 全文搜索（工作区内）
    async fn search_by_title(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
        search_term: &str,
    ) -> Result<Vec<Issue>, AppError>;

    /// 创建 Issue
    async fn insert(
        &self,
        conn: &mut PgConnection,
        new_issue: &NewIssue,
    ) -> Result<Issue, AppError>;

    /// 按 ID 删除
    async fn delete_by_id(
        &self,
        conn: &mut PgConnection,
        issue_id: Uuid,
    ) -> Result<usize, AppError>;

    /// 获取下一个 Issue 编号
    async fn get_next_issue_number(
        &self,
        conn: &mut PgConnection,
        team_id: Uuid,
    ) -> Result<i32, AppError>;
}

/// Workspace Member 仓储 trait
#[async_trait]
pub trait WorkspaceMemberRepositoryTrait: Send + Sync {
    /// 查找用户在工作区的成员关系
    async fn find(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<crate::db::models::workspace_member::WorkspaceMember>, AppError>;

    /// 添加成员
    async fn insert(
        &self,
        conn: &mut PgConnection,
        new_member: &crate::db::models::workspace_member::NewWorkspaceMember,
    ) -> Result<crate::db::models::workspace_member::WorkspaceMember, AppError>;

    /// 列出工作区所有成员
    async fn list_by_workspace(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
    ) -> Result<Vec<crate::db::models::workspace_member::WorkspaceMember>, AppError>;

    /// 删除成员
    async fn delete(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
        user_id: Uuid,
    ) -> Result<usize, AppError>;
}

/// User 仓储 trait
#[async_trait]
pub trait UserRepositoryTrait: Send + Sync {
    /// 按 ID 查找用户
    async fn find_by_id(
        &self,
        conn: &mut PgConnection,
        user_id: Uuid,
    ) -> Result<Option<crate::db::models::User>, AppError>;

    /// 按 email 查找
    async fn find_by_email(
        &self,
        conn: &mut PgConnection,
        email: &str,
    ) -> Result<Option<crate::db::models::User>, AppError>;

    /// 更新用户当前工作区
    async fn update_current_workspace(
        &self,
        conn: &mut PgConnection,
        user_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<crate::db::models::User, AppError>;
}

// 实现 IssueRepo -> IssueRepositoryTrait 的桥接
pub struct IssueRepoAdapter;

#[async_trait]
impl IssueRepositoryTrait for IssueRepoAdapter {
    async fn find_by_id_in_workspace(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
        issue_id: Uuid,
    ) -> Result<Option<Issue>, AppError> {
        super::issues::IssueRepo::find_by_id_in_workspace(conn, workspace_id, issue_id)
            .map_err(AppError::Database)
    }

    async fn list_by_workspace(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
    ) -> Result<Vec<Issue>, AppError> {
        super::issues::IssueRepo::list_by_workspace(conn, workspace_id)
            .map_err(AppError::Database)
    }

    async fn list_by_team(
        &self,
        conn: &mut PgConnection,
        team_id: Uuid,
        limit: i64,
        cursor: Option<IssueCursor>,
    ) -> Result<Vec<Issue>, AppError> {
        super::issues::IssueRepo::list_by_team(conn, team_id, limit, cursor)
            .map_err(AppError::Database)
    }

    async fn list_by_team_filtered(
        &self,
        conn: &mut PgConnection,
        team_id: Option<Uuid>,
        project_id: Option<Uuid>,
        assignee_id: Option<Uuid>,
        priority: Option<String>,
        search: Option<String>,
        limit: i64,
        cursor: Option<IssueCursor>,
    ) -> Result<Vec<Issue>, AppError> {
        super::issues::IssueRepo::list_by_team_filtered(
            conn,
            team_id,
            project_id,
            assignee_id,
            priority,
            search,
            limit,
            cursor,
        )
        .map_err(AppError::Database)
    }

    async fn search_by_title(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
        search_term: &str,
    ) -> Result<Vec<Issue>, AppError> {
        super::issues::IssueRepo::search_by_title(conn, workspace_id, search_term)
            .map_err(AppError::Database)
    }

    async fn insert(
        &self,
        conn: &mut PgConnection,
        new_issue: &NewIssue,
    ) -> Result<Issue, AppError> {
        super::issues::IssueRepo::insert(conn, new_issue).map_err(AppError::Database)
    }

    async fn delete_by_id(
        &self,
        conn: &mut PgConnection,
        issue_id: Uuid,
    ) -> Result<usize, AppError> {
        super::issues::IssueRepo::delete_by_id(conn, issue_id).map_err(AppError::Database)
    }

    async fn get_next_issue_number(
        &self,
        conn: &mut PgConnection,
        team_id: Uuid,
    ) -> Result<i32, AppError> {
        super::issues::IssueRepo::get_next_issue_number(conn, team_id).map_err(AppError::Database)
    }
}

/// WorkspaceMemberRepo 适配器
pub struct WorkspaceMemberRepoAdapter;

#[async_trait]
impl WorkspaceMemberRepositoryTrait for WorkspaceMemberRepoAdapter {
    async fn find(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<crate::db::models::workspace_member::WorkspaceMember>, AppError> {
        super::workspace_members::WorkspaceMembersRepo::find(conn, workspace_id, user_id)
            .map_err(AppError::Database)
    }

    async fn insert(
        &self,
        conn: &mut PgConnection,
        new_member: &crate::db::models::workspace_member::NewWorkspaceMember,
    ) -> Result<crate::db::models::workspace_member::WorkspaceMember, AppError> {
        super::workspace_members::WorkspaceMembersRepo::insert(conn, new_member)
            .map_err(AppError::Database)
    }

    async fn list_by_workspace(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
    ) -> Result<Vec<crate::db::models::workspace_member::WorkspaceMember>, AppError> {
        super::workspace_members::WorkspaceMembersRepo::list_by_workspace(conn, workspace_id)
            .map_err(AppError::Database)
    }

    async fn delete(
        &self,
        conn: &mut PgConnection,
        workspace_id: Uuid,
        user_id: Uuid,
    ) -> Result<usize, AppError> {
        super::workspace_members::WorkspaceMembersRepo::delete(conn, workspace_id, user_id)
            .map_err(AppError::Database)
    }
}

/// UserRepo 适配器
pub struct UserRepoAdapter;

#[async_trait]
impl UserRepositoryTrait for UserRepoAdapter {
    async fn find_by_id(
        &self,
        conn: &mut PgConnection,
        user_id: Uuid,
    ) -> Result<Option<crate::db::models::User>, AppError> {
        super::auth::AuthRepo::find_by_id(conn, user_id).map_err(AppError::Database)
    }

    async fn find_by_email(
        &self,
        conn: &mut PgConnection,
        email: &str,
    ) -> Result<Option<crate::db::models::User>, AppError> {
        // find_by_email 实际定义在 auth repo 中
        super::auth::AuthRepo::find_by_email(conn, email).map_err(AppError::Database)
    }

    async fn update_current_workspace(
        &self,
        conn: &mut PgConnection,
        user_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<crate::db::models::User, AppError> {
        super::auth::AuthRepo::update_current_workspace(conn, user_id, workspace_id)
            .map_err(AppError::Database)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 简单的 mock 实现示例（生产中可使用 mockall）
    pub struct MockIssueRepo {
        pub next_id: std::sync::atomic::AtomicU64,
    }

    #[async_trait]
    impl IssueRepositoryTrait for MockIssueRepo {
        async fn find_by_id_in_workspace(
            &self,
            _conn: &mut PgConnection,
            _workspace_id: Uuid,
            _issue_id: Uuid,
        ) -> Result<Option<Issue>, AppError> {
            Ok(None)
        }

        async fn list_by_workspace(
            &self,
            _conn: &mut PgConnection,
            _workspace_id: Uuid,
        ) -> Result<Vec<Issue>, AppError> {
            Ok(vec![])
        }

        async fn list_by_team(
            &self,
            _conn: &mut PgConnection,
            _team_id: Uuid,
            _limit: i64,
            _cursor: Option<IssueCursor>,
        ) -> Result<Vec<Issue>, AppError> {
            Ok(vec![])
        }

        async fn list_by_team_filtered(
            &self,
            _conn: &mut PgConnection,
            _team_id: Option<Uuid>,
            _project_id: Option<Uuid>,
            _assignee_id: Option<Uuid>,
            _priority: Option<String>,
            _search: Option<String>,
            _limit: i64,
            _cursor: Option<IssueCursor>,
        ) -> Result<Vec<Issue>, AppError> {
            Ok(vec![])
        }

        async fn search_by_title(
            &self,
            _conn: &mut PgConnection,
            _workspace_id: Uuid,
            _search_term: &str,
        ) -> Result<Vec<Issue>, AppError> {
            Ok(vec![])
        }

        async fn insert(
            &self,
            _conn: &mut PgConnection,
            _new_issue: &NewIssue,
        ) -> Result<Issue, AppError> {
            // Mock: 真实实现需要 Diesel Insertable，单元测试中简化
            Err(AppError::Internal("MockIssueRepo::insert not implemented".into()))
        }

        async fn delete_by_id(
            &self,
            _conn: &mut PgConnection,
            _issue_id: Uuid,
        ) -> Result<usize, AppError> {
            Ok(1)
        }

        async fn get_next_issue_number(
            &self,
            _conn: &mut PgConnection,
            _team_id: Uuid,
        ) -> Result<i32, AppError> {
            Ok(1)
        }
    }

    #[tokio::test]
    async fn test_mock_issue_repo_works() {
        let mock = MockIssueRepo {
            next_id: std::sync::atomic::AtomicU64::new(0),
        };

        // 验证 trait 可以被 mock
        let _: Box<dyn IssueRepositoryTrait> = Box::new(mock);
    }
}