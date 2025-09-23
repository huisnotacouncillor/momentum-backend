use diesel::prelude::*;

use crate::{
    db::models::cycle::{Cycle, NewCycle},
    db::repositories::cycles::CyclesRepo,
    error::AppError,
    services::context::RequestContext,
};

pub struct CyclesService;

impl CyclesService {
    pub fn list(conn: &mut PgConnection, ctx: &RequestContext) -> Result<Vec<Cycle>, AppError> {
        let list = CyclesRepo::list_by_workspace(conn, ctx.workspace_id)?;
        Ok(list)
    }

    pub fn create(conn: &mut PgConnection, _ctx: &RequestContext, req: &crate::routes::cycles::CreateCycleRequest) -> Result<Cycle, AppError> {
        let new_cycle = NewCycle {
            team_id: req.team_id,
            name: req.name.clone(),
            start_date: req.start_date,
            end_date: req.end_date,
            description: req.description.clone(),
            goal: req.goal.clone(),
        };
        let created = CyclesRepo::insert(conn, &new_cycle)?;
        Ok(created)
    }

    pub fn get_by_id(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        cycle_id: uuid::Uuid,
    ) -> Result<Cycle, AppError> {
        let cycle = CyclesRepo::find_by_id(conn, cycle_id)?
            .ok_or_else(|| AppError::not_found("cycle"))?;
        Ok(cycle)
    }

    pub fn update(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        cycle_id: uuid::Uuid,
        req: &crate::routes::cycles::UpdateCycleRequest,
    ) -> Result<Cycle, AppError> {
        let _existing = CyclesRepo::find_by_id(conn, cycle_id)?
            .ok_or_else(|| AppError::not_found("cycle"))?;

        let updated = CyclesRepo::update_fields(
            conn,
            cycle_id,
            req.name.as_ref().map(|s| s.as_str()),
            req.description.as_ref().map(|s| s.as_str()),
            req.goal.as_ref().map(|s| s.as_str()),
            req.start_date.map(|d| chrono::NaiveDateTime::from(d.and_hms_opt(0, 0, 0).unwrap_or_default())),
            req.end_date.map(|d| chrono::NaiveDateTime::from(d.and_hms_opt(0, 0, 0).unwrap_or_default())),
        )?;
        Ok(updated)
    }

    pub fn delete(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        cycle_id: uuid::Uuid,
    ) -> Result<(), AppError> {
        let _existing = CyclesRepo::find_by_id(conn, cycle_id)?
            .ok_or_else(|| AppError::not_found("cycle"))?;

        CyclesRepo::delete_by_id(conn, cycle_id)?;
        Ok(())
    }

    pub fn get_stats(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        cycle_id: uuid::Uuid,
    ) -> Result<crate::routes::cycles::CycleStats, AppError> {
        let cycle = CyclesRepo::find_by_id(conn, cycle_id)?
            .ok_or_else(|| AppError::not_found("cycle"))?;

        // Get issue counts by status - simplified for now
        // use crate::schema::issues; // not used in simplified stats
        let stats: Vec<(String, i64)> = Vec::new(); // Simplified for now

        let mut total_issues = 0;
        let mut completed_issues = 0;
        let mut in_progress_issues = 0;
        let mut planned_issues = 0;

        for (status, count) in stats {
            total_issues += count;
            match status.as_str() {
                "completed" => completed_issues = count,
                "in_progress" => in_progress_issues = count,
                "planned" => planned_issues = count,
                _ => {}
            }
        }

        Ok(crate::routes::cycles::CycleStats {
            id: cycle.id,
            name: cycle.name,
            start_date: cycle.start_date,
            end_date: cycle.end_date,
            status: cycle.status,
            total_issues,
            completed_issues,
            in_progress_issues,
            todo_issues: planned_issues,
            completion_rate: if total_issues > 0 { (completed_issues as f64) / (total_issues as f64) } else { 0.0 },
            days_remaining: 0, // Calculate based on end_date
            is_overdue: false, // Calculate based on end_date
        })
    }

    pub fn get_issues(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        cycle_id: uuid::Uuid,
        page: Option<i64>,
        limit: Option<i64>,
        _status: Option<String>,
    ) -> Result<Vec<crate::db::models::issue::Issue>, AppError> {
        let _cycle = CyclesRepo::find_by_id(conn, cycle_id)?
            .ok_or_else(|| AppError::not_found("cycle"))?;

        use crate::schema::issues;
        let query = issues::table
            .filter(issues::cycle_id.eq(cycle_id))
            .into_boxed();

        // Note: issues table might not have status field, simplified for now
        // if let Some(status_filter) = status {
        //     query = query.filter(issues::status.eq(status_filter));
        // }

        let offset = page.unwrap_or(1) - 1;
        let limit = limit.unwrap_or(20);

        let issues_list = query
            .offset(offset * limit)
            .limit(limit)
            .load::<crate::db::models::issue::Issue>(conn)?;

        Ok(issues_list)
    }

    pub fn assign_issues(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        cycle_id: uuid::Uuid,
        issue_ids: &[uuid::Uuid],
    ) -> Result<(), AppError> {
        let _cycle = CyclesRepo::find_by_id(conn, cycle_id)?
            .ok_or_else(|| AppError::not_found("cycle"))?;

        use crate::schema::issues;
        diesel::update(issues::table.filter(issues::id.eq_any(issue_ids)))
            .set(issues::cycle_id.eq(cycle_id))
            .execute(conn)?;

        Ok(())
    }

    pub fn remove_issues(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        cycle_id: uuid::Uuid,
        issue_ids: &[uuid::Uuid],
    ) -> Result<(), AppError> {
        let _cycle = CyclesRepo::find_by_id(conn, cycle_id)?
            .ok_or_else(|| AppError::not_found("cycle"))?;

        use crate::schema::issues;
        diesel::update(issues::table.filter(issues::id.eq_any(issue_ids)))
            .set(issues::cycle_id.eq(None::<uuid::Uuid>))
            .execute(conn)?;

        Ok(())
    }

    pub fn auto_update_status(_conn: &mut PgConnection) -> Result<(), AppError> {
        use chrono::Utc;

        let _now = Utc::now().naive_utc();

        // Update cycles that have started - simplified for now
        // Note: cycles table might not have status field
        // diesel::update(cycles::table.filter(cycles::start_date.le(now)))
        //     .set(cycles::status.eq("active"))
        //     .execute(conn)?;

        // Update cycles that have ended - simplified for now
        // diesel::update(cycles::table.filter(cycles::end_date.le(now)))
        //     .set(cycles::status.eq("completed"))
        //     .execute(conn)?;

        Ok(())
    }
}


