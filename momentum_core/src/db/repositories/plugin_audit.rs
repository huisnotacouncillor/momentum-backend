use diesel::prelude::*;
use uuid::Uuid;

use crate::db::models::plugin_audit::NewPluginAudit;
use crate::schema::plugin_audit;

pub struct PluginAuditRepo;

impl PluginAuditRepo {
    pub fn record(
        conn: &mut PgConnection,
        entry: &NewPluginAudit,
    ) -> Result<(), diesel::result::Error> {
        diesel::insert_into(plugin_audit::table)
            .values(entry)
            .execute(conn)?;
        Ok(())
    }

    pub fn list_by_plugin(
        conn: &mut PgConnection,
        p_id: &str,
        limit: i64,
    ) -> Result<Vec<crate::db::models::plugin_audit::PluginAuditRow>, diesel::result::Error> {
        plugin_audit::table
            .filter(plugin_audit::plugin_id.eq(p_id))
            .order(plugin_audit::created_at.desc())
            .limit(limit)
            .load::<crate::db::models::plugin_audit::PluginAuditRow>(conn)
    }

    pub fn list_by_workspace(
        conn: &mut PgConnection,
        ws_id: Uuid,
        limit: i64,
    ) -> Result<Vec<crate::db::models::plugin_audit::PluginAuditRow>, diesel::result::Error> {
        plugin_audit::table
            .filter(plugin_audit::workspace_id.eq(ws_id))
            .order(plugin_audit::created_at.desc())
            .limit(limit)
            .load::<crate::db::models::plugin_audit::PluginAuditRow>(conn)
    }
}
