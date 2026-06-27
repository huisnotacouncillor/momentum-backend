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
        _workspace_id: uuid::Uuid,
    ) -> Result<Vec<Issue>, diesel::result::Error> {
        use crate::schema::issues::dsl::*;
        issues.order(created_at.desc()).load::<Issue>(conn)
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

        // Pre-compute search pattern strings to avoid lifetime issues with boxed queries
        let (title_pattern, desc_pattern) = if let Some(ref s) = p_search {
            (Some(format!("%{}%", s)), Some(format!("%{}%", s)))
        } else {
            (None, None)
        };

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
        if let Some(ref tp) = title_pattern {
            if let Some(ref dp) = desc_pattern {
                query = query.filter(
                    title.ilike(tp.as_str()).or(description.ilike(dp.as_str())),
                );
            }
        }

        if let Some(cur) = cursor {
            query = query
                .filter(created_at.lt(cur.created_at))
                .or_filter(created_at.eq(cur.created_at).and(id.lt(cur.id)));
        }

        query
            .order((created_at.desc().nulls_last(), id.desc().nulls_last()))
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
        _workspace_id: uuid::Uuid,
        search_term: &str,
    ) -> Result<Vec<Issue>, diesel::result::Error> {
        use crate::schema::issues::dsl::*;
        let pattern = format!("%{}%", search_term);
        issues
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
        _workspace_id: uuid::Uuid,
        issue_id: uuid::Uuid,
    ) -> Result<Option<Issue>, diesel::result::Error> {
        use crate::schema::issues::dsl::*;
        issues
            .filter(id.eq(issue_id))
            .first::<Issue>(conn)
            .optional()
    }
}
