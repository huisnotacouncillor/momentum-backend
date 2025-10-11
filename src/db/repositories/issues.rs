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

    #[allow(clippy::type_complexity)]
    pub fn update_fields(
        conn: &mut PgConnection,
        issue_id: uuid::Uuid,
        changes: (
            Option<String>,
            Option<String>,
            Option<uuid::Uuid>,
            Option<uuid::Uuid>,
            Option<String>,
            Option<uuid::Uuid>,
            Option<uuid::Uuid>,
            Option<uuid::Uuid>,
            Option<uuid::Uuid>,
        ),
    ) -> Result<Issue, diesel::result::Error> {
        use crate::schema::issues::dsl as i;

        // Handle partial updates - this is simplified, in practice you'd want more granular control
        if let Some(title_val) = changes.0.clone() {
            return diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(i::title.eq(title_val))
                .get_result(conn);
        }

        if let Some(description_val) = changes.1.clone() {
            return diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(i::description.eq(description_val))
                .get_result(conn);
        }

        if let Some(project_id_val) = changes.2 {
            return diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(i::project_id.eq(project_id_val))
                .get_result(conn);
        }

        if let Some(team_id_val) = changes.3 {
            return diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(i::team_id.eq(team_id_val))
                .get_result(conn);
        }

        if let Some(priority_val) = changes.4.clone() {
            return diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(i::priority.eq(priority_val))
                .get_result(conn);
        }

        if let Some(assignee_id_val) = changes.5 {
            return diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(i::assignee_id.eq(assignee_id_val))
                .get_result(conn);
        }

        if let Some(workflow_id_val) = changes.6 {
            return diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(i::workflow_id.eq(workflow_id_val))
                .get_result(conn);
        }

        if let Some(workflow_state_id_val) = changes.7 {
            return diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(i::workflow_state_id.eq(workflow_state_id_val))
                .get_result(conn);
        }

        if let Some(cycle_id_val) = changes.8 {
            return diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(i::cycle_id.eq(cycle_id_val))
                .get_result(conn);
        }

        // No changes provided, return current row
        use crate::schema::issues::dsl::*;
        issues.filter(id.eq(issue_id)).first::<Issue>(conn)
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
