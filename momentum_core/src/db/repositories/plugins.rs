use diesel::prelude::*;

use crate::db::models::plugin::{NewPlugin, Plugin};
use crate::schema::plugins;

pub struct PluginRepo;

impl PluginRepo {
    pub fn upsert(
        conn: &mut PgConnection,
        new_p: &NewPlugin,
    ) -> Result<Plugin, diesel::result::Error> {
        diesel::insert_into(plugins::table)
            .values(new_p)
            .on_conflict(plugins::id)
            .do_update()
            .set((
                plugins::version.eq(&new_p.version),
                plugins::manifest.eq(&new_p.manifest),
                plugins::status.eq(&new_p.status),
                plugins::updated_at.eq(chrono::Utc::now()),
            ))
            .get_result(conn)
    }

    pub fn find_by_id(
        conn: &mut PgConnection,
        p_id: &str,
    ) -> Result<Option<Plugin>, diesel::result::Error> {
        plugins::table
            .filter(plugins::id.eq(p_id))
            .first::<Plugin>(conn)
            .optional()
    }

    pub fn list_available(conn: &mut PgConnection) -> Result<Vec<Plugin>, diesel::result::Error> {
        plugins::table
            .filter(plugins::status.eq("available"))
            .load::<Plugin>(conn)
    }
}
