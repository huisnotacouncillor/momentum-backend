//! 扩展点 7：Storage Namespace
//!
//! 插件的隔离 KV 存储，namespace 隔离。
//! 详见 docs/PLUGIN_SDK_DESIGN.md §3 / §7

use diesel::PgConnection;
use uuid::Uuid;

use crate::db::repositories::plugin_storage::PluginStorageRepo;
use crate::plugins::error::{PluginError, PluginResult};
use crate::plugins::manifest::Manifest;
use crate::plugins::permission::check_permission;

pub struct StorageService;

impl StorageService {
    pub fn get(
        conn: &mut PgConnection,
        plugin_id: &str,
        workspace_id: Uuid,
        manifest: &Manifest,
        namespace: &str,
        key: &str,
    ) -> PluginResult<Option<serde_json::Value>> {
        check_namespace(manifest, namespace)?;
        let perm = format!("storage.read:{}", namespace);
        check_permission(manifest, &perm)?;

        Ok(PluginStorageRepo::get(
            conn,
            plugin_id,
            workspace_id,
            namespace,
            key,
        )?)
    }

    pub fn put(
        conn: &mut PgConnection,
        plugin_id: &str,
        workspace_id: Uuid,
        manifest: &Manifest,
        namespace: &str,
        key: &str,
        value: &serde_json::Value,
    ) -> PluginResult<()> {
        check_namespace(manifest, namespace)?;
        let perm = format!("storage.write:{}", namespace);
        check_permission(manifest, &perm)?;

        PluginStorageRepo::put(conn, plugin_id, workspace_id, namespace, key, value)?;
        Ok(())
    }

    pub fn delete(
        conn: &mut PgConnection,
        plugin_id: &str,
        workspace_id: Uuid,
        manifest: &Manifest,
        namespace: &str,
        key: &str,
    ) -> PluginResult<()> {
        check_namespace(manifest, namespace)?;
        let perm = format!("storage.write:{}", namespace);
        check_permission(manifest, &perm)?;

        PluginStorageRepo::delete(conn, plugin_id, workspace_id, namespace, key)?;
        Ok(())
    }
}

fn check_namespace(manifest: &Manifest, namespace: &str) -> PluginResult<()> {
    let declared = manifest
        .extensions
        .storage
        .iter()
        .any(|s| s.namespace == namespace);
    if !declared {
        return Err(PluginError::PermissionDenied(format!(
            "namespace '{}' not declared in manifest.storage",
            namespace
        )));
    }
    Ok(())
}
