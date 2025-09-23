use diesel::prelude::*;

use crate::{
    db::models::workflow::{Workflow, NewWorkflow, WorkflowState, NewWorkflowState},
    db::repositories::workflows::WorkflowsRepo,
    error::AppError,
    services::context::RequestContext,
    validation::workflow::{validate_create_workflow, validate_create_state},
};

pub struct WorkflowsService;

impl WorkflowsService {
    pub fn list_by_team(conn: &mut PgConnection, _ctx: &RequestContext, team_id: uuid::Uuid) -> Result<Vec<Workflow>, AppError> {
        let list = WorkflowsRepo::list_by_team(conn, team_id)?;
        Ok(list)
    }

    pub fn create_workflow(conn: &mut PgConnection, _ctx: &RequestContext, team_id: uuid::Uuid, name: &str, description: Option<String>, is_default: bool) -> Result<Workflow, AppError> {
        validate_create_workflow(name)?;
        let new_wf = NewWorkflow { name: name.to_string(), description, team_id, is_default };
        let wf = WorkflowsRepo::insert_workflow(conn, &new_wf)?;
        Ok(wf)
    }

    pub fn add_state(conn: &mut PgConnection, _ctx: &RequestContext, workflow_id: uuid::Uuid, req: &crate::db::models::workflow::CreateWorkflowStateRequest) -> Result<WorkflowState, AppError> {
        validate_create_state(&req.name, req.position)?;
        let new_state = NewWorkflowState {
            workflow_id,
            name: req.name.clone(),
            description: req.description.clone(),
            color: req.color.clone(),
            category: req.category,
            position: req.position,
            is_default: req.is_default.unwrap_or(false),
        };
        let st = WorkflowsRepo::insert_state(conn, &new_state)?;
        Ok(st)
    }

    pub fn list(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        team_id: uuid::Uuid,
    ) -> Result<Vec<Workflow>, AppError> {
        let list = WorkflowsRepo::list_by_team(conn, team_id)?;
        Ok(list)
    }

    pub fn get_by_id(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        workflow_id: uuid::Uuid,
    ) -> Result<Workflow, AppError> {
        let workflow = WorkflowsRepo::find_by_id(conn, workflow_id)?
            .ok_or_else(|| AppError::not_found("workflow"))?;
        Ok(workflow)
    }

    pub fn update(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        workflow_id: uuid::Uuid,
        req: &crate::routes::workflows::UpdateWorkflowRequest,
    ) -> Result<Workflow, AppError> {
        let _existing = WorkflowsRepo::find_by_id(conn, workflow_id)?
            .ok_or_else(|| AppError::not_found("workflow"))?;

        let updated = WorkflowsRepo::update_fields(
            conn,
            workflow_id,
            req.name.as_ref().map(|s| s.as_str()),
            req.description.as_ref().map(|s| s.as_str()),
            req.is_default,
        )?;
        Ok(updated)
    }

    pub fn delete(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        workflow_id: uuid::Uuid,
    ) -> Result<(), AppError> {
        let _existing = WorkflowsRepo::find_by_id(conn, workflow_id)?
            .ok_or_else(|| AppError::not_found("workflow"))?;

        WorkflowsRepo::delete_by_id(conn, workflow_id)?;
        Ok(())
    }

    pub fn get_states(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        workflow_id: uuid::Uuid,
    ) -> Result<Vec<WorkflowState>, AppError> {
        let _workflow = WorkflowsRepo::find_by_id(conn, workflow_id)?
            .ok_or_else(|| AppError::not_found("workflow"))?;

        let states = WorkflowsRepo::list_states_by_workflow(conn, workflow_id)?;
        Ok(states)
    }

    pub fn get_team_default_states(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        team_id: uuid::Uuid,
    ) -> Result<Vec<WorkflowState>, AppError> {
        let states = WorkflowsRepo::list_team_default_states(conn, team_id)?;
        Ok(states)
    }

    pub fn create_team_default_state(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        team_id: uuid::Uuid,
        req: &crate::routes::workflows::CreateTeamDefaultStateRequest,
    ) -> Result<WorkflowState, AppError> {
        validate_create_state(&req.name, req.position)?;
        let new_state = NewWorkflowState {
            workflow_id: uuid::Uuid::new_v4(), // This will be handled by the repo
            name: req.name.clone(),
            description: req.description.clone(),
            color: req.color.clone(),
            category: req.category,
            position: req.position,
            is_default: true,
        };
        let state = WorkflowsRepo::insert_team_default_state(conn, team_id, &new_state)?;
        Ok(state)
    }

    pub fn update_team_default_state(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        team_id: uuid::Uuid,
        state_id: uuid::Uuid,
        req: &crate::routes::workflows::UpdateTeamDefaultStateRequest,
    ) -> Result<WorkflowState, AppError> {
        let _existing = WorkflowsRepo::find_team_default_state_by_id(conn, team_id, state_id)?
            .ok_or_else(|| AppError::not_found("team_default_state"))?;

        let updated = WorkflowsRepo::update_team_default_state_fields(
            conn,
            state_id,
            req.name.as_ref().map(|s| s.as_str()),
            req.description.as_ref().map(|s| s.as_str()),
            req.color.as_ref().map(|s| s.as_str()),
            req.category.as_ref(),
            req.position,
        )?;
        Ok(updated)
    }

    pub fn get_issue_transitions(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        issue_id: uuid::Uuid,
    ) -> Result<Vec<crate::db::models::workflow::IssueTransitionResponse>, AppError> {
        // This would typically involve complex logic to determine valid transitions
        // For now, return a simple implementation
        use crate::schema::{issues, workflows, workflow_states};

        let issue = issues::table
            .filter(issues::id.eq(issue_id))
            .first::<crate::db::models::issue::Issue>(conn)
            .optional()?
            .ok_or_else(|| AppError::not_found("issue"))?;

        // Get all possible states for the workflow
        let states = workflow_states::table
            .inner_join(workflows::table.on(workflows::id.eq(workflow_states::workflow_id)))
            .filter(workflows::team_id.eq(issue.team_id))
            .load::<(WorkflowState, Workflow)>(conn)?;

        // Enrich with from/to state objects
        let mut result = Vec::new();
        for (state, _workflow) in states {
            let from_state = match issue.workflow_state_id {
                Some(sid) => workflow_states::table
                    .filter(workflow_states::id.eq(sid))
                    .first::<WorkflowState>(conn)
                    .optional()
                    .map_err(|e| AppError::internal(&format!("Failed to load from_state: {}", e)))?
                    .map(|s| crate::db::models::workflow::WorkflowStateResponse::from(s)),
                None => None,
            };

            result.push(crate::db::models::workflow::IssueTransitionResponse {
                id: uuid::Uuid::new_v4(),
                workflow_id: state.workflow_id,
                from_state_id: issue.workflow_state_id,
                to_state_id: state.id,
                name: None,
                description: None,
                created_at: chrono::Utc::now(),
                from_state,
                to_state: crate::db::models::workflow::WorkflowStateResponse::from(state),
            });
        }

        Ok(result)
    }
}


