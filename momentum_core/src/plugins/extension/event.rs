//! 扩展点 6：Webhook Bus
//!
//! 插件 publish 自定义事件（写入 outbox + audit）。
//! subscribe 由 Registry 推流到插件进程（v0.2 接入 NATS 后实现）。

use diesel::ExpressionMethods;
use diesel::PgConnection;
use diesel::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::plugin_audit::NewPluginAudit;
use crate::db::repositories::plugin_audit::PluginAuditRepo;
use crate::plugins::audit::events as ev;
use crate::plugins::error::{PluginError, PluginResult};
use crate::plugins::manifest::Manifest;
use crate::plugins::permission::check_permission;
use crate::schema::outbox;

pub struct EventService;

impl EventService {
    /// 插件发布事件（写入 outbox + audit）
    pub fn publish(
        conn: &mut PgConnection,
        plugin_id: &str,
        workspace_id: Uuid,
        actor_id: Option<Uuid>,
        manifest: &Manifest,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> PluginResult<()> {
        // 1. 校验 manifest 声明了 publishes
        if !manifest
            .extensions
            .webhooks
            .publishes
            .iter()
            .any(|e| e == event_type)
        {
            return Err(PluginError::PermissionDenied(format!(
                "event '{}' not in manifest.webhooks.publishes",
                event_type
            )));
        }
        // 2. 权限检查
        let perm = format!("event.publish:{}", event_type);
        check_permission(manifest, &perm)?;

        // 3. 写 outbox
        diesel::insert_into(outbox::table)
            .values((
                outbox::aggregate_type.eq(Some(plugin_id.to_string())),
                outbox::aggregate_id.eq(Some(workspace_id)),
                outbox::event_type.eq(event_type.to_string()),
                outbox::payload.eq(payload.clone()),
            ))
            .execute(conn)?;

        // 4. 审计
        PluginAuditRepo::record(
            conn,
            &NewPluginAudit {
                plugin_id: plugin_id.to_string(),
                workspace_id: Some(workspace_id),
                event: ev::EVENT_PUBLISH.replace("event.publish", event_type),
                payload: Some(payload.clone()),
                actor_id,
            },
        )?;

        Ok(())
    }

    /// 审计：事件订阅（v0.2 真正接 NATS 后再实现）
    #[allow(dead_code)]
    pub fn subscribe(manifest: &Manifest, event_type: &str) -> PluginResult<()> {
        if !manifest
            .extensions
            .webhooks
            .subscribes
            .iter()
            .any(|e| e == event_type)
        {
            return Err(PluginError::PermissionDenied(format!(
                "event '{}' not in manifest.webhooks.subscribes",
                event_type
            )));
        }
        Ok(())
    }
}
