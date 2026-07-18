//! Team repository - 数据库访问封装
//!
//! 提供 team 数据的 CRUD 操作，封装 Diesel schema DSL 调用。

use diesel::prelude::*;
use uuid::Uuid;

use crate::db::models::team::{NewTeam, Team};
use crate::error::AppError;
use crate::schema::teams::dsl as t;

pub struct TeamRepo;

impl TeamRepo {
    /// 按 ID 查找（工作区隔离）
    pub fn find_by_id(
        conn: &mut PgConnection,
        workspace_id: Uuid,
        team_id: Uuid,
    ) -> Result<Option<Team>, AppError> {
        match t::teams
            .filter(t::id.eq(team_id))
            .filter(t::workspace_id.eq(workspace_id))
            .select(Team::as_select())
            .first::<Team>(conn)
        {
            Ok(team) => Ok(Some(team)),
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    /// 按工作区列出所有团队
    pub fn list_by_workspace(
        conn: &mut PgConnection,
        workspace_id: Uuid,
    ) -> Result<Vec<Team>, AppError> {
        t::teams
            .filter(t::workspace_id.eq(workspace_id))
            .select(Team::as_select())
            .order(t::created_at.desc())
            .load::<Team>(conn)
            .map_err(AppError::Database)
    }

    /// 创建团队
    pub fn insert(conn: &mut PgConnection, new_team: &NewTeam) -> Result<Team, AppError> {
        diesel::insert_into(t::teams)
            .values(new_team)
            .get_result::<Team>(conn)
            .map_err(AppError::Database)
    }

    /// 更新团队
    pub fn update(
        conn: &mut PgConnection,
        team_id: Uuid,
        updates: Team,
    ) -> Result<Team, AppError> {
        diesel::update(t::teams.filter(t::id.eq(team_id)))
            .set((
                t::name.eq(updates.name),
                t::team_key.eq(updates.team_key),
                t::description.eq(updates.description),
                t::icon_url.eq(updates.icon_url),
                t::is_private.eq(updates.is_private),
            ))
            .get_result::<Team>(conn)
            .map_err(AppError::Database)
    }

    /// 按 ID 删除
    pub fn delete_by_id(conn: &mut PgConnection, team_id: Uuid) -> Result<usize, AppError> {
        diesel::delete(t::teams.filter(t::id.eq(team_id)))
            .execute(conn)
            .map_err(AppError::Database)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn team_repo_compiles() {
        // 验证模块可编译
        use super::TeamRepo;
        let _ = TeamRepo;
    }
}
