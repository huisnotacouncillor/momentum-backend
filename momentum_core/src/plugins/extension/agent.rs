//! 扩展点 3：Agent SDK（业务封装层）
//!
//! 真正调用插件进程的工作在 momentum_plugin_host crate。
//! core 这边负责 agent_run 生命周期的 DB 记录 + manifest 校验。
//!
//! 详见 docs/PLUGIN_SDK_DESIGN.md §3

use diesel::PgConnection;
use uuid::Uuid;

use crate::db::models::agent_run::NewAgentRun;
use crate::db::repositories::agent_runs::AgentRunRepo;
use crate::plugins::error::{PluginError, PluginResult};
use crate::plugins::manifest::Manifest;

pub struct AgentService;

impl AgentService {
    /// 在调用 gRPC 之前创建一条 "running" 状态的 agent_run 记录
    pub fn start_run(
        conn: &mut PgConnection,
        workspace_id: Uuid,
        issue_id: Option<Uuid>,
        manifest: &Manifest,
        agent_id: &str,
        input: &serde_json::Value,
        actor_id: Option<Uuid>,
    ) -> PluginResult<Uuid> {
        // 校验 agent 在 manifest 里声明过
        if !manifest.extensions.agents.iter().any(|a| a.id == agent_id) {
            return Err(PluginError::AgentNotRegistered(format!(
                "{}.{}",
                manifest.id, agent_id
            )));
        }

        let new_run = NewAgentRun {
            workspace_id,
            issue_id,
            plugin_id: manifest.id.clone(),
            agent_id: agent_id.to_string(),
            status: "running".to_string(),
            input: Some(input.clone()),
            actor_id,
        };
        let run = AgentRunRepo::insert(conn, &new_run)?;
        Ok(run.id)
    }

    pub fn complete_run(
        conn: &mut PgConnection,
        run_id: Uuid,
        output: &serde_json::Value,
        tokens_in: Option<i32>,
        tokens_out: Option<i32>,
        duration_ms: Option<i32>,
    ) -> PluginResult<()> {
        AgentRunRepo::complete(
            conn,
            run_id,
            "succeeded",
            Some(output),
            tokens_in,
            tokens_out,
            duration_ms,
        )?;
        Ok(())
    }

    pub fn fail_run(
        conn: &mut PgConnection,
        run_id: Uuid,
        error_msg: &str,
        duration_ms: Option<i32>,
    ) -> PluginResult<()> {
        AgentRunRepo::fail(conn, run_id, error_msg, duration_ms)?;
        Ok(())
    }

    /// 列出最近 runs（给 UI 看）
    pub fn recent_runs(
        conn: &mut PgConnection,
        workspace_id: Uuid,
        limit: i64,
    ) -> PluginResult<Vec<crate::db::models::agent_run::AgentRun>> {
        Ok(AgentRunRepo::list_by_workspace(conn, workspace_id, limit)?)
    }
}
