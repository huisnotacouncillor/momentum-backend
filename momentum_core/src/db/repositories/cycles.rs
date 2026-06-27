use diesel::prelude::*;

use crate::db::models::cycle::{Cycle, NewCycle};

pub struct CyclesRepo;

impl CyclesRepo {
    pub fn insert(
        conn: &mut PgConnection,
        new_cycle: &NewCycle,
    ) -> Result<Cycle, diesel::result::Error> {
        diesel::insert_into(crate::schema::cycles::table)
            .values(new_cycle)
            .get_result(conn)
    }

    pub fn list_by_workspace(
        conn: &mut PgConnection,
        ws_id: uuid::Uuid,
    ) -> Result<Vec<Cycle>, diesel::result::Error> {
        use crate::schema::{cycles, teams};
        cycles::table
            .inner_join(teams::table.on(cycles::team_id.eq(teams::id)))
            .filter(teams::workspace_id.eq(ws_id))
            .select(Cycle::as_select())
            .order(cycles::start_date.desc())
            .load::<Cycle>(conn)
    }

    pub fn find_by_id_in_workspace(
        conn: &mut PgConnection,
        ws_id: uuid::Uuid,
        cycle_id: uuid::Uuid,
    ) -> Result<Option<Cycle>, diesel::result::Error> {
        use crate::schema::{cycles, teams};
        cycles::table
            .inner_join(teams::table.on(cycles::team_id.eq(teams::id)))
            .filter(cycles::id.eq(cycle_id))
            .filter(teams::workspace_id.eq(ws_id))
            .select(Cycle::as_select())
            .first::<Cycle>(conn)
            .optional()
    }

    pub fn delete_by_id(
        conn: &mut PgConnection,
        cycle_id_val: uuid::Uuid,
    ) -> Result<usize, diesel::result::Error> {
        use crate::schema::cycles::dsl::*;
        diesel::delete(cycles.filter(id.eq(cycle_id_val))).execute(conn)
    }

    pub fn find_by_id(
        conn: &mut PgConnection,
        cycle_id: uuid::Uuid,
    ) -> Result<Option<Cycle>, diesel::result::Error> {
        use crate::schema::cycles::dsl::*;
        cycles
            .filter(id.eq(cycle_id))
            .first::<Cycle>(conn)
            .optional()
    }

    pub fn update_fields(
        conn: &mut PgConnection,
        cycle_id: uuid::Uuid,
        name: Option<&str>,
        description: Option<&str>,
        goal: Option<&str>,
        start_date: Option<chrono::NaiveDateTime>,
        end_date: Option<chrono::NaiveDateTime>,
    ) -> Result<Cycle, diesel::result::Error> {
        use crate::schema::cycles::dsl as c;

        // Update each field individually
        if let Some(name_val) = name {
            diesel::update(c::cycles.filter(c::id.eq(cycle_id)))
                .set(c::name.eq(name_val))
                .execute(conn)?;
        }
        if let Some(desc) = description {
            diesel::update(c::cycles.filter(c::id.eq(cycle_id)))
                .set(c::description.eq(desc))
                .execute(conn)?;
        }
        if let Some(goal_val) = goal {
            diesel::update(c::cycles.filter(c::id.eq(cycle_id)))
                .set(c::goal.eq(goal_val))
                .execute(conn)?;
        }
        if let Some(start) = start_date {
            diesel::update(c::cycles.filter(c::id.eq(cycle_id)))
                .set(c::start_date.eq(start.date()))
                .execute(conn)?;
        }
        if let Some(end) = end_date {
            diesel::update(c::cycles.filter(c::id.eq(cycle_id)))
                .set(c::end_date.eq(end.date()))
                .execute(conn)?;
        }

        // Return the updated cycle
        c::cycles.filter(c::id.eq(cycle_id)).first::<Cycle>(conn)
    }
}
