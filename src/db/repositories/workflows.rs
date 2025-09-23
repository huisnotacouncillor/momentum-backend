use diesel::prelude::*;

use crate::db::models::workflow::{Workflow, NewWorkflow, WorkflowState, NewWorkflowState};

pub struct WorkflowsRepo;

impl WorkflowsRepo {
    pub fn list_by_team(conn: &mut PgConnection, team: uuid::Uuid) -> Result<Vec<Workflow>, diesel::result::Error> {
        use crate::schema::workflows::dsl::*;
        workflows.filter(team_id.eq(team)).order(created_at.desc()).load::<Workflow>(conn)
    }

    pub fn insert_workflow(conn: &mut PgConnection, new_wf: &NewWorkflow) -> Result<Workflow, diesel::result::Error> {
        diesel::insert_into(crate::schema::workflows::table).values(new_wf).get_result(conn)
    }

    pub fn find_workflow(conn: &mut PgConnection, wf_id: uuid::Uuid) -> Result<Option<Workflow>, diesel::result::Error> {
        use crate::schema::workflows::dsl::*;
        workflows.filter(id.eq(wf_id)).first::<Workflow>(conn).optional()
    }

    pub fn delete_workflow(conn: &mut PgConnection, wf_id: uuid::Uuid) -> Result<usize, diesel::result::Error> {
        use crate::schema::workflows::dsl::*;
        diesel::delete(workflows.filter(id.eq(wf_id))).execute(conn)
    }

    pub fn list_states(conn: &mut PgConnection, wf: uuid::Uuid) -> Result<Vec<WorkflowState>, diesel::result::Error> {
        use crate::schema::workflow_states::dsl::*;
        workflow_states.filter(workflow_id.eq(wf)).order(position.asc()).load::<WorkflowState>(conn)
    }

    pub fn insert_state(conn: &mut PgConnection, new_state: &NewWorkflowState) -> Result<WorkflowState, diesel::result::Error> {
        diesel::insert_into(crate::schema::workflow_states::table).values(new_state).get_result(conn)
    }

    pub fn find_by_id(conn: &mut PgConnection, workflow_id: uuid::Uuid) -> Result<Option<Workflow>, diesel::result::Error> {
        use crate::schema::workflows::dsl::*;
        workflows.filter(id.eq(workflow_id)).first::<Workflow>(conn).optional()
    }

    pub fn update_fields(
        conn: &mut PgConnection,
        workflow_id: uuid::Uuid,
        name: Option<&str>,
        description: Option<&str>,
        is_default: Option<bool>,
    ) -> Result<Workflow, diesel::result::Error> {
        use crate::schema::workflows::dsl as w;

        // Update each field individually
        if let Some(name_val) = name {
            diesel::update(w::workflows.filter(w::id.eq(workflow_id)))
                .set(w::name.eq(name_val))
                .execute(conn)?;
        }
        if let Some(desc) = description {
            diesel::update(w::workflows.filter(w::id.eq(workflow_id)))
                .set(w::description.eq(desc))
                .execute(conn)?;
        }
        if let Some(default) = is_default {
            diesel::update(w::workflows.filter(w::id.eq(workflow_id)))
                .set(w::is_default.eq(default))
                .execute(conn)?;
        }

        // Return the updated workflow
        w::workflows.filter(w::id.eq(workflow_id)).first::<Workflow>(conn)
    }

    pub fn delete_by_id(conn: &mut PgConnection, workflow_id: uuid::Uuid) -> Result<usize, diesel::result::Error> {
        use crate::schema::workflows::dsl::*;
        diesel::delete(workflows.filter(id.eq(workflow_id))).execute(conn)
    }

    pub fn list_states_by_workflow(conn: &mut PgConnection, wf: uuid::Uuid) -> Result<Vec<WorkflowState>, diesel::result::Error> {
        use crate::schema::workflow_states::dsl::*;
        workflow_states.filter(workflow_id.eq(wf)).order(position.asc()).load::<WorkflowState>(conn)
    }

    pub fn list_team_default_states(conn: &mut PgConnection, team: uuid::Uuid) -> Result<Vec<WorkflowState>, diesel::result::Error> {
        use crate::schema::{workflow_states, workflows};
        workflow_states::table
            .inner_join(workflows::table.on(workflow_states::workflow_id.eq(workflows::id)))
            .filter(workflows::team_id.eq(team))
            .filter(workflow_states::is_default.eq(true))
            .select(WorkflowState::as_select())
            .order(workflow_states::position.asc())
            .load::<WorkflowState>(conn)
    }

    pub fn insert_team_default_state(conn: &mut PgConnection, _team_id: uuid::Uuid, new_state: &NewWorkflowState) -> Result<WorkflowState, diesel::result::Error> {
        // For now, just insert the state as-is
        // In a real implementation, this might create a workflow first
        diesel::insert_into(crate::schema::workflow_states::table).values(new_state).get_result(conn)
    }

    pub fn find_team_default_state_by_id(conn: &mut PgConnection, team: uuid::Uuid, state_id: uuid::Uuid) -> Result<Option<WorkflowState>, diesel::result::Error> {
        use crate::schema::{workflow_states, workflows};
        workflow_states::table
            .inner_join(workflows::table.on(workflow_states::workflow_id.eq(workflows::id)))
            .filter(workflows::team_id.eq(team))
            .filter(workflow_states::id.eq(state_id))
            .filter(workflow_states::is_default.eq(true))
            .select(WorkflowState::as_select())
            .first::<WorkflowState>(conn)
            .optional()
    }

    pub fn update_team_default_state_fields(
        conn: &mut PgConnection,
        state_id: uuid::Uuid,
        name: Option<&str>,
        description: Option<&str>,
        color: Option<&str>,
        category: Option<&crate::db::models::workflow::WorkflowStateCategory>,
        position: Option<i32>,
    ) -> Result<WorkflowState, diesel::result::Error> {
        use crate::schema::workflow_states::dsl as ws;

        // Update each field individually
        if let Some(name_val) = name {
            diesel::update(ws::workflow_states.filter(ws::id.eq(state_id)))
                .set(ws::name.eq(name_val))
                .execute(conn)?;
        }
        if let Some(desc) = description {
            diesel::update(ws::workflow_states.filter(ws::id.eq(state_id)))
                .set(ws::description.eq(desc))
                .execute(conn)?;
        }
        if let Some(color_val) = color {
            diesel::update(ws::workflow_states.filter(ws::id.eq(state_id)))
                .set(ws::color.eq(color_val))
                .execute(conn)?;
        }
        if let Some(cat) = category {
            diesel::update(ws::workflow_states.filter(ws::id.eq(state_id)))
                .set(ws::category.eq(cat))
                .execute(conn)?;
        }
        if let Some(pos) = position {
            diesel::update(ws::workflow_states.filter(ws::id.eq(state_id)))
                .set(ws::position.eq(pos))
                .execute(conn)?;
        }

        // Return the updated workflow state
        ws::workflow_states.filter(ws::id.eq(state_id)).first::<WorkflowState>(conn)
    }
}


