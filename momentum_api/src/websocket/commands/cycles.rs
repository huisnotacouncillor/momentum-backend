//! Cycle command handlers

use std::sync::Arc;
use uuid::Uuid;
use momentum_core::db::DbPool;
use momentum_core::error::AppError;
use momentum_core::services::cycles_service::CyclesService;
use super::types::*;

pub struct CycleHandlers;

impl CycleHandlers {
    pub async fn handle_query_cycles(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;
        let cycles = CyclesService::list(&mut conn, &ctx)?;
        Ok(serde_json::json!(cycles))
    }

    pub async fn handle_get_cycle(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
        cycle_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;
        let cycle = CyclesService::get_by_id(&mut conn, &ctx, cycle_id)?;
        Ok(serde_json::json!(cycle))
    }

    pub async fn handle_create_cycle(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
        data: CreateCycleCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;
        let cycle = CyclesService::create(&mut conn, &ctx, &data.into())?;
        Ok(serde_json::json!(cycle))
    }

    pub async fn handle_update_cycle(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
        cycle_id: Uuid,
        data: UpdateCycleCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;
        let cycle = CyclesService::update(&mut conn, &ctx, cycle_id, &data.into())?;
        Ok(serde_json::json!(cycle))
    }

    pub async fn handle_delete_cycle(
        db: &Arc<DbPool>,
        ctx: momentum_core::services::context::RequestContext,
        cycle_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = db.get()?;
        CyclesService::delete(&mut conn, &ctx, cycle_id)?;
        Ok(serde_json::json!({ "deleted": true, "cycle_id": cycle_id }))
    }
}

// Convert websocket command types to service types
impl From<CreateCycleCommand> for momentum_core::services::cycles::types::CreateCycleRequest {
    fn from(cmd: CreateCycleCommand) -> Self {
        Self {
            team_id: cmd.team_id,
            name: cmd.name,
            // Default to today if not provided
            start_date: cmd.start_date.unwrap_or_else(|| chrono::Local::now().date_naive()),
            end_date: cmd.end_date.unwrap_or_else(|| chrono::Local::now().date_naive()),
            description: cmd.description,
            goal: cmd.goal,
        }
    }
}

impl From<UpdateCycleCommand> for momentum_core::services::cycles::types::UpdateCycleRequest {
    fn from(cmd: UpdateCycleCommand) -> Self {
        Self {
            team_id: None,
            name: cmd.name,
            start_date: cmd.start_date,
            end_date: cmd.end_date,
            status: cmd.status,
            description: cmd.description,
            goal: cmd.goal,
        }
    }
}
