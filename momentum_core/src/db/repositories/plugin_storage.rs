use diesel::prelude::*;
use uuid::Uuid;

use crate::db::models::plugin_storage::{NewPluginStorage, PluginStorageChangeset};
use crate::schema::plugin_storage;

pub struct PluginStorageRepo;

impl PluginStorageRepo {
    pub fn get(
        conn: &mut PgConnection,
        p_id: &str,
        ws_id: Uuid,
        ns: &str,
        k: &str,
    ) -> Result<Option<serde_json::Value>, diesel::result::Error> {
        let row: Option<serde_json::Value> = plugin_storage::table
            .filter(plugin_storage::plugin_id.eq(p_id))
            .filter(plugin_storage::workspace_id.eq(ws_id))
            .filter(plugin_storage::namespace.eq(ns))
            .filter(plugin_storage::key.eq(k))
            .select(plugin_storage::value)
            .first::<serde_json::Value>(conn)
            .optional()?;
        Ok(row)
    }

    pub fn put(
        conn: &mut PgConnection,
        p_id: &str,
        ws_id: Uuid,
        ns: &str,
        k: &str,
        v: &serde_json::Value,
    ) -> Result<(), diesel::result::Error> {
        let new_entry = NewPluginStorage {
            plugin_id: p_id.to_string(),
            workspace_id: ws_id,
            namespace: ns.to_string(),
            key: k.to_string(),
            value: v.clone(),
        };
        diesel::insert_into(plugin_storage::table)
            .values(&new_entry)
            .on_conflict((
                plugin_storage::plugin_id,
                plugin_storage::workspace_id,
                plugin_storage::namespace,
                plugin_storage::key,
            ))
            .do_update()
            .set(PluginStorageChangeset {
                value: v.clone(),
                updated_at: chrono::Utc::now(),
            })
            .execute(conn)?;
        Ok(())
    }

    pub fn delete(
        conn: &mut PgConnection,
        p_id: &str,
        ws_id: Uuid,
        ns: &str,
        k: &str,
    ) -> Result<usize, diesel::result::Error> {
        diesel::delete(
            plugin_storage::table
                .filter(plugin_storage::plugin_id.eq(p_id))
                .filter(plugin_storage::workspace_id.eq(ws_id))
                .filter(plugin_storage::namespace.eq(ns))
                .filter(plugin_storage::key.eq(k)),
        )
        .execute(conn)
    }
}
