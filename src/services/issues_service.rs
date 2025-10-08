use chrono::Utc;
use diesel::prelude::*;
use uuid::Uuid;

use crate::{
    db::enums::IssuePriority,
    db::models::issue::{Issue, NewIssue},
    db::models::team::{Team, TeamBasicInfo},
    db::models::workflow::WorkflowStateResponse,
    db::repositories::issues::IssueRepo,
    db::repositories::workflows::WorkflowsRepo,
    error::AppError,
    services::context::RequestContext,
    validation::issue::{validate_create_issue, validate_update_issue},
};

pub struct IssuesService;

impl IssuesService {
    fn priority_to_string(priority: &IssuePriority) -> String {
        match priority {
            IssuePriority::None => "none".to_string(),
            IssuePriority::Low => "low".to_string(),
            IssuePriority::Medium => "medium".to_string(),
            IssuePriority::High => "high".to_string(),
            IssuePriority::Urgent => "urgent".to_string(),
        }
    }
    pub fn list(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        filters: &IssueFilters,
    ) -> Result<Vec<crate::db::models::issue::IssueResponse>, AppError> {
        let mut query = IssueRepo::list_by_workspace(conn, ctx.workspace_id)?;

        // Apply filters
        if let Some(team_id) = filters.team_id {
            query.retain(|issue| issue.team_id == team_id);
        }

        if let Some(project_id) = filters.project_id {
            query.retain(|issue| issue.project_id == Some(project_id));
        }

        if let Some(assignee_id) = filters.assignee_id {
            query.retain(|issue| issue.assignee_id == Some(assignee_id));
        }

        if let Some(priority) = &filters.priority {
            let pri = Self::priority_to_string(priority);
            query.retain(|issue| issue.priority == pri);
        }

        if let Some(search) = &filters.search {
            query.retain(|issue| issue.title.to_lowercase().contains(&search.to_lowercase()));
        }

        // Enrich with workflow states and map to response
        let mut responses = Vec::with_capacity(query.len());
        for issue in query {
            let mut resp = crate::db::models::issue::IssueResponse::from(issue.clone());
            // Populate team info (and team_key)
            {
                use crate::schema::teams::dsl as t;
                if let Some(team) = t::teams
                    .filter(t::id.eq(issue.team_id))
                    .first::<Team>(conn)
                    .optional()
                    .map_err(|e| AppError::internal(&format!("Failed to load team: {}", e)))?
                {
                    resp.team_key = Some(team.team_key.clone());
                    resp.team = Some(TeamBasicInfo {
                        id: team.id,
                        name: team.name,
                        team_key: team.team_key,
                        description: team.description,
                        icon_url: team.icon_url,
                        is_private: team.is_private,
                    });
                }
            }
            // Determine states source
            let states = if let Some(wf_id) = issue.workflow_id {
                WorkflowsRepo::list_states_by_workflow(conn, wf_id).map_err(|e| {
                    AppError::internal(&format!("Failed to load workflow states: {}", e))
                })?
            } else {
                WorkflowsRepo::list_team_default_states(conn, issue.team_id).map_err(|e| {
                    AppError::internal(&format!("Failed to load team default states: {}", e))
                })?
            };
            resp.workflow_states = states
                .into_iter()
                .map(WorkflowStateResponse::from)
                .collect();
            responses.push(resp);
        }

        Ok(responses)
    }

    pub fn create(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        req: &crate::routes::issues::CreateIssueRequest,
    ) -> Result<Issue, AppError> {
        validate_create_issue(&req.title, &req.description, &req.team_id)?;

        let _now = Utc::now().naive_utc();
        let new_issue = NewIssue {
            project_id: req.project_id,
            cycle_id: req.cycle_id,
            creator_id: ctx.user_id,
            assignee_id: req.assignee_id,
            parent_issue_id: req.parent_issue_id,
            title: req.title.clone(),
            description: req.description.clone(),
            priority: req.priority.as_ref().map(|p| Self::priority_to_string(p)),
            is_changelog_candidate: Some(false),
            team_id: req.team_id,
            workflow_id: req.workflow_id,
            workflow_state_id: req.workflow_state_id,
        };

        IssueRepo::insert(conn, &new_issue)
            .map_err(|e| AppError::internal(&format!("Failed to create issue: {}", e)))
    }

    pub fn update(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        issue_id: Uuid,
        changes: &crate::routes::issues::UpdateIssueRequest,
    ) -> Result<Issue, AppError> {
        // Only validate title/description when provided
        if changes.title.is_some() || changes.description.is_some() {
            validate_update_issue(&changes.title, &changes.description)?;
        }

        // Ensure issue exists in workspace
        let existing = IssueRepo::find_by_id_in_workspace(conn, ctx.workspace_id, issue_id)?
            .ok_or_else(|| AppError::not_found("issue"))?;

        // Build changeset
        let mut cs = crate::db::models::issue::UpdateIssue::default();

        if let Some(t) = &changes.title {
            cs.title = Some(t.clone());
        }
        if let Some(d) = &changes.description {
            cs.description = Some(Some(d.clone()));
        }
        if let Some(pid) = changes.project_id {
            cs.project_id = Some(Some(pid));
        }
        if let Some(tid) = changes.team_id {
            cs.team_id = Some(tid);
        }
        if let Some(aid) = changes.assignee_id {
            cs.assignee_id = Some(Some(aid));
        }
        if let Some(cyc) = changes.cycle_id {
            cs.cycle_id = Some(Some(cyc));
        }
        if let Some(pr) = &changes.priority {
            cs.priority = Some(Self::priority_to_string(pr));
        }

        // Handle workflow/workflow_state validation and setting
        use crate::schema::{workflow_states as ws, workflows as w};
        use diesel::prelude::*;

        // Validate foreign keys belong to current workspace/team as appropriate
        {
            use crate::schema::{cycles, projects, teams, users};
            use diesel::prelude::*;

            if let Some(tid) = changes.team_id {
                // team must be in current workspace
                let owner_ws: Option<(uuid::Uuid,)> = teams::dsl::teams
                    .filter(teams::dsl::id.eq(tid))
                    .select((teams::dsl::workspace_id,))
                    .first::<(uuid::Uuid,)>(conn)
                    .optional()
                    .map_err(|e| AppError::internal(&format!("Failed to validate team: {}", e)))?;
                match owner_ws {
                    Some((ws_id,)) if ws_id == ctx.workspace_id => {}
                    _ => return Err(AppError::validation("Invalid team_id for workspace")),
                }
            }

            if let Some(pid) = changes.project_id {
                // project must be in current workspace
                let proj_ws: Option<(uuid::Uuid,)> = projects::dsl::projects
                    .filter(projects::dsl::id.eq(pid))
                    .select((projects::dsl::workspace_id,))
                    .first::<(uuid::Uuid,)>(conn)
                    .optional()
                    .map_err(|e| {
                        AppError::internal(&format!("Failed to validate project: {}", e))
                    })?;
                match proj_ws {
                    Some((ws_id,)) if ws_id == ctx.workspace_id => {}
                    _ => return Err(AppError::validation("Invalid project_id for workspace")),
                }
            }

            if let Some(cyc_id) = changes.cycle_id {
                // cycle must belong to a team in current workspace
                use crate::schema::teams as t2;
                let ok: Option<(uuid::Uuid,)> = cycles::dsl::cycles
                    .inner_join(t2::dsl::teams.on(cycles::dsl::team_id.eq(t2::dsl::id)))
                    .filter(cycles::dsl::id.eq(cyc_id))
                    .filter(t2::dsl::workspace_id.eq(ctx.workspace_id))
                    .select((cycles::dsl::id,))
                    .first::<(uuid::Uuid,)>(conn)
                    .optional()
                    .map_err(|e| AppError::internal(&format!("Failed to validate cycle: {}", e)))?;
                if ok.is_none() {
                    return Err(AppError::validation("Invalid cycle_id for workspace"));
                }
            }

            if let Some(aid) = changes.assignee_id {
                // user must exist
                let exists: Option<(uuid::Uuid,)> = users::dsl::users
                    .filter(users::dsl::id.eq(aid))
                    .select((users::dsl::id,))
                    .first::<(uuid::Uuid,)>(conn)
                    .optional()
                    .map_err(|e| {
                        AppError::internal(&format!("Failed to validate assignee: {}", e))
                    })?;
                if exists.is_none() {
                    return Err(AppError::validation("Invalid assignee_id"));
                }
            }
        }
        if let Some(state_id) = changes.workflow_state_id {
            // Determine target workflow id for the state
            // Determine new team context if updated
            let team_for_validation = changes.team_id.unwrap_or(existing.team_id);

            if let Some(req_workflow_id) = changes.workflow_id.or(existing.workflow_id) {
                // Validate state belongs to this workflow
                let found: Option<(Uuid,)> = ws::dsl::workflow_states
                    .filter(ws::dsl::id.eq(state_id))
                    .filter(ws::dsl::workflow_id.eq(req_workflow_id))
                    .select((ws::dsl::workflow_id,))
                    .first::<(Uuid,)>(conn)
                    .optional()
                    .map_err(|e| {
                        AppError::internal(&format!("Failed to validate workflow state: {}", e))
                    })?;
                if found.is_none() {
                    return Err(AppError::validation(
                        "Invalid workflow_state_id for workflow",
                    ));
                }
                // Set workflow to match the state's workflow
                cs.workflow_id = Some(Some(req_workflow_id));
            } else {
                // Deduce by team
                let found: Option<(Uuid,)> = ws::dsl::workflow_states
                    .inner_join(w::dsl::workflows.on(w::dsl::id.eq(ws::dsl::workflow_id)))
                    .filter(ws::dsl::id.eq(state_id))
                    .filter(w::dsl::team_id.eq(team_for_validation))
                    .select((ws::dsl::workflow_id,))
                    .first::<(Uuid,)>(conn)
                    .optional()
                    .map_err(|e| {
                        AppError::internal(&format!("Failed to validate workflow state: {}", e))
                    })?;
                match found {
                    Some((wf_id,)) => {
                        cs.workflow_id = Some(Some(wf_id));
                    }
                    None => return Err(AppError::validation("Invalid workflow_state_id for team")),
                }
            }

            // Set state and workflow
            cs.workflow_state_id = Some(Some(state_id));
        } else if let Some(wf_id) = changes.workflow_id {
            // Only workflow provided; validate workflow belongs to (new) team
            use crate::schema::workflows as w;
            let team_for_validation = changes.team_id.unwrap_or(existing.team_id);
            let ok: Option<(Uuid,)> = w::dsl::workflows
                .filter(w::dsl::id.eq(wf_id))
                .filter(w::dsl::team_id.eq(team_for_validation))
                .select((w::dsl::id,))
                .first::<(Uuid,)>(conn)
                .optional()
                .map_err(|e| AppError::internal(&format!("Failed to validate workflow: {}", e)))?;
            if ok.is_none() {
                return Err(AppError::validation("Invalid workflow_id for team"));
            }
            cs.workflow_id = Some(Some(wf_id));
        }

        // If team_id is changed and no explicit workflow fields provided, clear workflow linkage to avoid cross-team mismatch
        if changes.team_id.is_some()
            && changes.workflow_id.is_none()
            && changes.workflow_state_id.is_none()
        {
            cs.workflow_id = Some(None);
            cs.workflow_state_id = Some(None);
        }

        // Handle labels replacement if provided
        if let Some(ref label_ids) = changes.label_ids {
            use crate::schema::{issue_labels as il, labels as l};
            use diesel::prelude::*;

            // Validate all labels exist in current workspace
            if !label_ids.is_empty() {
                let count = l::dsl::labels
                    .filter(l::dsl::workspace_id.eq(ctx.workspace_id))
                    .filter(l::dsl::id.eq_any(label_ids))
                    .count()
                    .get_result::<i64>(conn)
                    .map_err(|e| {
                        AppError::internal(&format!("Failed to validate labels: {}", e))
                    })?;
                if count != label_ids.len() as i64 {
                    return Err(AppError::validation("Invalid label_ids for workspace"));
                }
            }

            // Replace issue labels
            diesel::delete(il::dsl::issue_labels.filter(il::dsl::issue_id.eq(issue_id)))
                .execute(conn)
                .map_err(|e| AppError::internal(&format!("Failed to clear issue labels: {}", e)))?;

            if !label_ids.is_empty() {
                let new_rows: Vec<crate::db::models::issue::NewIssueLabel> = label_ids
                    .iter()
                    .map(|lid| crate::db::models::issue::NewIssueLabel {
                        issue_id,
                        label_id: *lid,
                    })
                    .collect();
                diesel::insert_into(il::dsl::issue_labels)
                    .values(&new_rows)
                    .execute(conn)
                    .map_err(|e| {
                        AppError::internal(&format!("Failed to insert issue labels: {}", e))
                    })?;
            }
        }

        // Apply update using changeset only if any field changed; otherwise return current issue
        let has_field_changes = changes.title.is_some()
            || changes.description.is_some()
            || changes.project_id.is_some()
            || changes.team_id.is_some()
            || changes.assignee_id.is_some()
            || changes.cycle_id.is_some()
            || changes.priority.is_some()
            || changes.workflow_id.is_some()
            || changes.workflow_state_id.is_some();

        if has_field_changes {
            use crate::schema::issues::dsl as i;
            let updated = diesel::update(i::issues.filter(i::id.eq(issue_id)))
                .set(&cs)
                .get_result::<Issue>(conn)
                .map_err(|e| AppError::internal(&format!("Failed to update issue: {}", e)))?;
            Ok(updated)
        } else {
            // No changes to issue table; return current row
            let current = IssueRepo::find_by_id_in_workspace(conn, ctx.workspace_id, issue_id)?
                .ok_or_else(|| AppError::not_found("issue"))?;
            Ok(current)
        }
    }

    pub fn delete(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        issue_id: Uuid,
    ) -> Result<(), AppError> {
        // Ensure issue exists in workspace
        let existing = IssueRepo::find_by_id_in_workspace(conn, ctx.workspace_id, issue_id)?;
        if existing.is_none() {
            return Err(AppError::not_found("issue"));
        }

        IssueRepo::delete_by_id(conn, issue_id)
            .map_err(|e| AppError::internal(&format!("Failed to delete issue: {}", e)))?;

        Ok(())
    }

    pub fn get_by_id(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        issue_id: Uuid,
    ) -> Result<crate::db::models::issue::IssueResponse, AppError> {
        let issue = IssueRepo::find_by_id_in_workspace(conn, ctx.workspace_id, issue_id)?
            .ok_or_else(|| AppError::not_found("issue"))?;

        let mut resp = crate::db::models::issue::IssueResponse::from(issue.clone());

        // team info + team_key
        {
            use crate::schema::teams::dsl as t;
            if let Some(team) = t::teams
                .filter(t::id.eq(issue.team_id))
                .first::<Team>(conn)
                .optional()
                .map_err(|e| AppError::internal(&format!("Failed to load team: {}", e)))?
            {
                resp.team_key = Some(team.team_key.clone());
                resp.team = Some(TeamBasicInfo {
                    id: team.id,
                    name: team.name,
                    team_key: team.team_key,
                    description: team.description,
                    icon_url: team.icon_url,
                    is_private: team.is_private,
                });
            }
        }

        // project info
        if let Some(proj_id) = issue.project_id {
            use crate::schema::projects::dsl as p;
            if let Some(project) = p::projects
                .filter(p::id.eq(proj_id))
                .first::<crate::db::models::project::Project>(conn)
                .optional()
                .map_err(|e| AppError::internal(&format!("Failed to load project: {}", e)))?
            {
                // Build ProjectInfo
                // load status
                use crate::schema::project_statuses::dsl as ps;
                let status = ps::project_statuses
                    .filter(ps::id.eq(project.project_status_id))
                    .first::<crate::db::models::project_status::ProjectStatus>(conn)
                    .map_err(|e| {
                        AppError::internal(&format!("Failed to load project status: {}", e))
                    })?;
                // load owner basic info
                use crate::schema::users::dsl as u;
                let owner = u::users
                    .filter(u::id.eq(project.owner_id))
                    .first::<crate::db::models::auth::User>(conn)
                    .map_err(|e| {
                        AppError::internal(&format!("Failed to load project owner: {}", e))
                    })?;

                // Get all available statuses for this workspace
                let all_statuses = ps::project_statuses
                    .filter(ps::workspace_id.eq(ctx.workspace_id))
                    .select(crate::db::models::project_status::ProjectStatus::as_select())
                    .load::<crate::db::models::project_status::ProjectStatus>(conn)
                    .map_err(|e| {
                        AppError::internal(&format!("Failed to load project statuses: {}", e))
                    })?;

                let available_statuses: Vec<crate::db::models::project_status::ProjectStatusInfo> =
                    all_statuses
                        .into_iter()
                        .map(|status| {
                            crate::db::models::project_status::ProjectStatusInfo::from(status)
                        })
                        .collect();

                resp.project = Some(crate::db::models::project::ProjectInfo {
                    id: project.id,
                    name: project.name,
                    project_key: project.project_key,
                    description: project.description,
                    status: crate::db::models::project_status::ProjectStatusInfo::from(status),
                    available_statuses,
                    owner: crate::db::models::auth::UserBasicInfo {
                        id: owner.id,
                        name: owner.name,
                        username: owner.username,
                        email: owner.email,
                        avatar_url: owner.avatar_url,
                    },
                    target_date: project.target_date,
                    priority: project.priority,
                    created_at: project.created_at,
                    updated_at: project.updated_at,
                });
            }
        }

        // assignee info
        if let Some(uid) = issue.assignee_id {
            use crate::schema::users::dsl as u;
            if let Some(user) = u::users
                .filter(u::id.eq(uid))
                .first::<crate::db::models::auth::User>(conn)
                .optional()
                .map_err(|e| AppError::internal(&format!("Failed to load assignee: {}", e)))?
            {
                resp.assignee = Some(crate::db::models::auth::UserBasicInfo {
                    id: user.id,
                    name: user.name,
                    username: user.username,
                    email: user.email,
                    avatar_url: user.avatar_url,
                });
            }
        }

        // parent issue
        if let Some(pid) = issue.parent_issue_id {
            if let Some(parent) = IssueRepo::find_by_id(conn, pid)? {
                resp.parent_issue = Some(Box::new(crate::db::models::issue::IssueResponse::from(
                    parent,
                )));
            }
        }

        // child issues
        {
            use crate::schema::issues::dsl as i;
            let children = i::issues
                .filter(i::parent_issue_id.eq(Some(issue.id)))
                .order(i::created_at.asc())
                .load::<Issue>(conn)
                .map_err(|e| AppError::internal(&format!("Failed to load child issues: {}", e)))?;
            resp.child_issues = children
                .into_iter()
                .map(crate::db::models::issue::IssueResponse::from)
                .collect();
        }

        // workflow states
        let states = if let Some(wf_id) = issue.workflow_id {
            WorkflowsRepo::list_states_by_workflow(conn, wf_id).map_err(|e| {
                AppError::internal(&format!("Failed to load workflow states: {}", e))
            })?
        } else {
            WorkflowsRepo::list_team_default_states(conn, issue.team_id).map_err(|e| {
                AppError::internal(&format!("Failed to load team default states: {}", e))
            })?
        };
        resp.workflow_states = states
            .into_iter()
            .map(WorkflowStateResponse::from)
            .collect();

        // labels
        {
            use crate::schema::{issue_labels as il, labels as l};
            let labels = il::dsl::issue_labels
                .inner_join(l::dsl::labels.on(il::dsl::label_id.eq(l::dsl::id)))
                .filter(il::dsl::issue_id.eq(issue.id))
                .select(l::dsl::labels::all_columns())
                .load::<crate::db::models::label::Label>(conn)
                .map_err(|e| AppError::internal(&format!("Failed to load labels: {}", e)))?;
            resp.labels = labels;
        }

        // cycle
        if let Some(cycle_id) = issue.cycle_id {
            use crate::schema::cycles::dsl as c;
            if let Some(cycle) = c::cycles
                .filter(c::id.eq(cycle_id))
                .first::<crate::db::models::cycle::Cycle>(conn)
                .optional()
                .map_err(|e| AppError::internal(&format!("Failed to load cycle: {}", e)))?
            {
                resp.cycle = Some(cycle);
            }
        }

        Ok(resp)
    }
}

#[derive(Debug)]
pub struct IssueFilters {
    pub team_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub priority: Option<IssuePriority>,
    pub search: Option<String>,
}
