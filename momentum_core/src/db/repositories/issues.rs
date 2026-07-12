use diesel::prelude::*;

use crate::db::models::issue::{Issue, IssueCursor, NewIssue};

pub struct IssueRepo;

impl IssueRepo {
    pub fn get_next_issue_number(
        conn: &mut PgConnection,
        _team_id: uuid::Uuid,
    ) -> Result<i32, diesel::result::Error> {
        use crate::schema::issues::dsl::*;

        // Get max issue_number for this team, with 0 as default if no issues exist
        let max_number = issues
            .filter(team_id.eq(_team_id))
            .select(issue_number)
            .order(issue_number.desc())
            .limit(1)
            .first::<i32>(conn)
            .optional()?
            .unwrap_or(0);

        Ok(max_number + 1)
    }

    pub fn find_by_id(
        conn: &mut PgConnection,
        issue_id: uuid::Uuid,
    ) -> Result<Option<Issue>, diesel::result::Error> {
        use crate::schema::issues::dsl::*;
        issues
            .filter(id.eq(issue_id))
            .first::<Issue>(conn)
            .optional()
    }

    pub fn list_by_workspace(
        conn: &mut PgConnection,
        workspace_id: uuid::Uuid,
    ) -> Result<Vec<Issue>, diesel::result::Error> {
        use crate::schema::{issues::dsl::*, teams};

        // 先获取该工作区的所有 team_id
        let workspace_team_ids: Vec<uuid::Uuid> = teams::table
            .filter(teams::workspace_id.eq(workspace_id))
            .select(teams::id)
            .load(conn)?;

        issues
            .filter(team_id.eq_any(&workspace_team_ids))
            .order(created_at.desc())
            .load::<Issue>(conn)
    }

    pub fn list_by_team(
        conn: &mut PgConnection,
        target_team_id: uuid::Uuid,
        limit: i64,
        cursor: Option<IssueCursor>,
    ) -> Result<Vec<Issue>, diesel::result::Error> {
        Self::list_by_team_filtered(
            conn,
            Some(target_team_id),
            None,
            None,
            None,
            None,
            limit,
            cursor,
        )
    }

    pub fn list_by_team_filtered(
        conn: &mut PgConnection,
        p_team_id: Option<uuid::Uuid>,
        p_project_id: Option<uuid::Uuid>,
        p_assignee_id: Option<uuid::Uuid>,
        p_priority: Option<String>,
        p_search: Option<String>,
        limit: i64,
        cursor: Option<IssueCursor>,
    ) -> Result<Vec<Issue>, diesel::result::Error> {
        use crate::schema::issues::dsl::*;
        use diesel::BoolExpressionMethods;

        let mut query = issues.into_boxed();

        if let Some(tid) = p_team_id {
            query = query.filter(team_id.eq(tid));
        }
        if let Some(pid) = p_project_id {
            query = query.filter(project_id.eq(pid));
        }
        if let Some(aid) = p_assignee_id {
            query = query.filter(assignee_id.eq(aid));
        }
        if let Some(p) = p_priority {
            query = query.filter(priority.eq(p));
        }
        if let Some(ref search) = p_search {
            // Issue #3 修复：搜索 query 改用预计算 `search_vector` 列而非每次重算 tsvector
            // 这样 PostgreSQL 能命中迁移时创建的 GIN 索引 `idx_issues_search_vector`。
            // 重算版本会让搜索变全表扫描 + 每次都 build tsvector。
            query = query.filter(
                diesel::dsl::sql::<diesel::sql_types::Bool>(SEARCH_PREDICATE_SQL)
                    .bind::<diesel::sql_types::Text, _>(search),
            );

            // 按相关性（ts_rank）降序排序，然后按创建时间降序
            let rank_expr = diesel::dsl::sql::<diesel::sql_types::Float>(SEARCH_RANK_SQL)
                .bind::<diesel::sql_types::Text, _>(search);
            query = query.order((rank_expr.desc(), created_at.desc()));
        } else {
            query = query.order(created_at.desc().nulls_last());
        }

        if let Some(cur) = cursor {
            query = query
                .filter(created_at.lt(cur.created_at))
                .or_filter(created_at.eq(cur.created_at).and(id.lt(cur.id)));
        }

        query
            .limit(limit + 1)
            .load::<Issue>(conn)
    }

    pub fn list_by_project(
        conn: &mut PgConnection,
        target_project_id: uuid::Uuid,
    ) -> Result<Vec<Issue>, diesel::result::Error> {
        use crate::schema::issues::dsl::*;
        issues
            .filter(project_id.eq(target_project_id))
            .order(created_at.desc())
            .load::<Issue>(conn)
    }

    pub fn list_by_assignee(
        conn: &mut PgConnection,
        target_assignee_id: uuid::Uuid,
    ) -> Result<Vec<Issue>, diesel::result::Error> {
        use crate::schema::issues::dsl::*;
        issues
            .filter(assignee_id.eq(target_assignee_id))
            .order(created_at.desc())
            .load::<Issue>(conn)
    }

    pub fn search_by_title(
        conn: &mut PgConnection,
        workspace_id: uuid::Uuid,
        search_term: &str,
    ) -> Result<Vec<Issue>, diesel::result::Error> {
        use crate::schema::{issues::dsl::*, teams};

        let workspace_team_ids: Vec<uuid::Uuid> = teams::table
            .filter(teams::workspace_id.eq(workspace_id))
            .select(teams::id)
            .load(conn)?;

        let pattern = format!("%{}%", search_term);
        issues
            .filter(team_id.eq_any(&workspace_team_ids))
            .filter(title.like(pattern))
            .order(created_at.desc())
            .load::<Issue>(conn)
    }

    pub fn insert(
        conn: &mut PgConnection,
        new_issue: &NewIssue,
    ) -> Result<Issue, diesel::result::Error> {
        diesel::insert_into(crate::schema::issues::table)
            .values(new_issue)
            .get_result(conn)
    }

    pub fn delete_by_id(
        conn: &mut PgConnection,
        issue_id: uuid::Uuid,
    ) -> Result<usize, diesel::result::Error> {
        use crate::schema::issues::dsl::*;
        diesel::delete(issues.filter(id.eq(issue_id))).execute(conn)
    }

    pub fn find_by_id_in_workspace(
        conn: &mut PgConnection,
        workspace_id: uuid::Uuid,
        issue_id: uuid::Uuid,
    ) -> Result<Option<Issue>, diesel::result::Error> {
        use crate::schema::{issues::dsl::*, teams};

        // 验证 issue 属于指定工作区
        let workspace_team_ids: Vec<uuid::Uuid> = teams::table
            .filter(teams::workspace_id.eq(workspace_id))
            .select(teams::id)
            .load(conn)?;

        issues
            .filter(id.eq(issue_id))
            .filter(team_id.eq_any(&workspace_team_ids))
            .first::<Issue>(conn)
            .optional()
    }
}

/// 全文搜索过滤 SQL（用于 Issue 列表 WHERE 子句）
///
/// 注意：必须使用 `search_vector` 列 + `@@` 运算符，这样 PostgreSQL 才能命中迁移时
/// 创建的 GIN 索引 `idx_issues_search_vector`。
/// **禁止**重写为从 title/description 重新构造 tsvector 再 `@@`，
/// 那会每次都重算 tsvector，导致全表扫描，绕开索引。
pub const SEARCH_PREDICATE_SQL: &str =
    "search_vector @@ websearch_to_tsquery('english', $1)";

pub const SEARCH_RANK_SQL: &str =
    "ts_rank(search_vector, websearch_to_tsquery('english', $1))";

#[cfg(test)]
mod search_sql_guard_tests {
    use super::*;

    /// 防退化守门测试：保证搜索 SQL 走 `search_vector` 列 + GIN 索引。
    /// 历史教训：早期版本重算 tsvector 表达式导致搜索变成全表扫描（Issue #3）。
    #[test]
    fn search_predicate_sql_uses_search_vector_column() {
        assert!(
            SEARCH_PREDICATE_SQL.contains("search_vector @@"),
            "SEARCH_PREDICATE_SQL must use search_vector column for GIN index. got: {}",
            SEARCH_PREDICATE_SQL
        );
        assert!(
            !SEARCH_PREDICATE_SQL.contains("to_tsvector('english', title)"),
            "SEARCH_PREDICATE_SQL must NOT recompute tsvector from title (defeats GIN). got: {}",
            SEARCH_PREDICATE_SQL
        );
    }

    #[test]
    fn search_rank_sql_uses_search_vector_column() {
        assert!(
            SEARCH_RANK_SQL.contains("search_vector"),
            "SEARCH_RANK_SQL must use search_vector column. got: {}",
            SEARCH_RANK_SQL
        );
        assert!(
            !SEARCH_RANK_SQL.contains("to_tsvector('english', title) ||"),
            "SEARCH_RANK_SQL must NOT recompute tsvector. got: {}",
            SEARCH_RANK_SQL
        );
    }

    /// 反向守门：扫描 SQL 常量定义，禁止出现会绕开 GIN 索引的重算表达式
    ///
    /// 不扫描整个文件：测试代码本身的断言字符串会包含"bad pattern"导致误报。
    #[test]
    fn sql_constants_do_not_recompute_tsvector() {
        assert!(
            !SEARCH_PREDICATE_SQL.contains("to_tsvector('english', title) ||")
                && !SEARCH_PREDICATE_SQL.contains("to_tsvector('english', description)"),
            "SEARCH_PREDICATE_SQL must NOT inline-recompute tsvector. got: {}",
            SEARCH_PREDICATE_SQL
        );
        assert!(
            !SEARCH_RANK_SQL.contains("to_tsvector('english', title) ||")
                && !SEARCH_RANK_SQL.contains("to_tsvector('english', description)"),
            "SEARCH_RANK_SQL must NOT inline-recompute tsvector. got: {}",
            SEARCH_RANK_SQL
        );
    }
}
