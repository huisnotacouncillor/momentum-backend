use diesel::prelude::*;
use uuid::Uuid;

use crate::db::models::agent_run::{AgentRun, NewAgentRun};
use crate::schema::agent_runs;

pub struct AgentRunRepo;

impl AgentRunRepo {
    pub fn insert(
        conn: &mut PgConnection,
        new_run: &NewAgentRun,
    ) -> Result<AgentRun, diesel::result::Error> {
        diesel::insert_into(agent_runs::table)
            .values(new_run)
            .get_result(conn)
    }

    pub fn find_by_id(
        conn: &mut PgConnection,
        run_id: Uuid,
    ) -> Result<Option<AgentRun>, diesel::result::Error> {
        agent_runs::table
            .filter(agent_runs::id.eq(run_id))
            .first::<AgentRun>(conn)
            .optional()
    }

    pub fn complete(
        conn: &mut PgConnection,
        run_id: Uuid,
        new_status: &str,
        output: Option<&serde_json::Value>,
        tokens_in: Option<i32>,
        tokens_out: Option<i32>,
        duration_ms: Option<i32>,
    ) -> Result<AgentRun, diesel::result::Error> {
        diesel::update(agent_runs::table.filter(agent_runs::id.eq(run_id)))
            .set((
                agent_runs::status.eq(new_status.to_string()),
                agent_runs::output.eq(output.cloned()),
                agent_runs::tokens_input.eq(tokens_in),
                agent_runs::tokens_output.eq(tokens_out),
                agent_runs::duration_ms.eq(duration_ms),
                agent_runs::completed_at.eq(chrono::Utc::now()),
            ))
            .get_result(conn)
    }

    pub fn fail(
        conn: &mut PgConnection,
        run_id: Uuid,
        error_msg: &str,
        duration_ms: Option<i32>,
    ) -> Result<AgentRun, diesel::result::Error> {
        diesel::update(agent_runs::table.filter(agent_runs::id.eq(run_id)))
            .set((
                agent_runs::status.eq("failed".to_string()),
                agent_runs::error.eq(error_msg.to_string()),
                agent_runs::duration_ms.eq(duration_ms),
                agent_runs::completed_at.eq(chrono::Utc::now()),
            ))
            .get_result(conn)
    }

    pub fn list_by_workspace(
        conn: &mut PgConnection,
        ws_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AgentRun>, diesel::result::Error> {
        agent_runs::table
            .filter(agent_runs::workspace_id.eq(ws_id))
            .order(agent_runs::started_at.desc())
            .limit(limit)
            .load::<AgentRun>(conn)
    }
}
