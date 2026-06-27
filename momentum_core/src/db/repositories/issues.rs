use diesel::prelude::*;

use crate::db::models::issue::{Issue, NewIssue};

pub struct IssueRepo;

impl IssueRepo {
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
    ) -> Result<Vec<Issue>, diesel::result::Error> {
        use crate::schema::issues::dsl::*;
        issues
            .filter(team_id.eq(target_team_id))
            .order(created_at.desc())
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
