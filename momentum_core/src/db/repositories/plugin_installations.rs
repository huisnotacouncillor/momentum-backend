use diesel::prelude::*;
use uuid::Uuid;

use crate::db::models::plugin_installation::{NewPluginInstallation, PluginInstallation};
use crate::schema::plugin_installations;

pub struct PluginInstallationRepo;

impl PluginInstallationRepo {
    pub fn insert(
        conn: &mut PgConnection,
        new_inst: &NewPluginInstallation,
    ) -> Result<PluginInstallation, diesel::result::Error> {
        diesel::insert_into(plugin_installations::table)
            .values(new_inst)
            .get_result(conn)
    }

    pub fn find(
        conn: &mut PgConnection,
        ws_id: Uuid,
        p_id: &str,
    ) -> Result<Option<PluginInstallation>, diesel::result::Error> {
        plugin_installations::table
            .filter(plugin_installations::workspace_id.eq(ws_id))
            .filter(plugin_installations::plugin_id.eq(p_id))
            .first::<PluginInstallation>(conn)
            .optional()
    }

    pub fn find_by_id(
        conn: &mut PgConnection,
        inst_id: Uuid,
    ) -> Result<Option<PluginInstallation>, diesel::result::Error> {
        plugin_installations::table
            .filter(plugin_installations::id.eq(inst_id))
            .first::<PluginInstallation>(conn)
            .optional()
    }

    pub fn list_by_workspace(
        conn: &mut PgConnection,
        ws_id: Uuid,
    ) -> Result<Vec<PluginInstallation>, diesel::result::Error> {
        plugin_installations::table
            .filter(plugin_installations::workspace_id.eq(ws_id))
            .load::<PluginInstallation>(conn)
    }

    pub fn list_enabled_by_workspace(
        conn: &mut PgConnection,
        ws_id: Uuid,
    ) -> Result<Vec<PluginInstallation>, diesel::result::Error> {
        plugin_installations::table
            .filter(plugin_installations::workspace_id.eq(ws_id))
            .filter(plugin_installations::status.eq("enabled"))
            .load::<PluginInstallation>(conn)
    }

    pub fn update_status(
        conn: &mut PgConnection,
        inst_id: Uuid,
        new_status: &str,
        enabled_at: Option<chrono::DateTime<chrono::Utc>>,
        error_msg: Option<&str>,
    ) -> Result<PluginInstallation, diesel::result::Error> {
        diesel::update(plugin_installations::table.filter(plugin_installations::id.eq(inst_id)))
            .set((
                plugin_installations::status.eq(new_status.to_string()),
                plugin_installations::enabled_at.eq(enabled_at),
                plugin_installations::error_message.eq(error_msg.map(|s| s.to_string())),
            ))
            .get_result(conn)
    }

    pub fn delete(
        conn: &mut PgConnection,
        ws_id: Uuid,
        p_id: &str,
    ) -> Result<usize, diesel::result::Error> {
        diesel::delete(
            plugin_installations::table
                .filter(plugin_installations::workspace_id.eq(ws_id))
                .filter(plugin_installations::plugin_id.eq(p_id)),
        )
        .execute(conn)
    }
}
