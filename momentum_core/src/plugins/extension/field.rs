//! 扩展点 1：Field Extension
//!
//! 注册 Issue 自定义字段；写值时（可选）让插件校验。
//! 详见 docs/PLUGIN_SDK_DESIGN.md §3 / §7.3-§7.4

use diesel::PgConnection;
use uuid::Uuid;

use crate::db::repositories::{
    issue_field_definitions::IssueFieldDefinitionRepo,
    issue_field_values::{IssueFieldValueRepo, value_to_text},
};
use crate::plugins::error::{PluginError, PluginResult};

pub struct FieldService;

impl FieldService {
    /// 写一个字段值（不经插件校验——上层决定要不要调 OnFieldWrite）
    pub fn write_value(
        conn: &mut PgConnection,
        issue_id: Uuid,
        workspace_id: Uuid,
        plugin_id: &str,
        field_key: &str,
        value: serde_json::Value,
    ) -> PluginResult<()> {
        let def = IssueFieldDefinitionRepo::find_by_key(conn, workspace_id, plugin_id, field_key)?
            .ok_or_else(|| {
                PluginError::FieldNotRegistered(format!("{}.{}", plugin_id, field_key))
            })?;

        let text = value_to_text(&value);
        IssueFieldValueRepo::upsert(conn, issue_id, def.id, value, text)?;
        Ok(())
    }

    /// 读一个 issue 的所有字段值
    pub fn read_values(
        conn: &mut PgConnection,
        issue_id: Uuid,
    ) -> PluginResult<std::collections::HashMap<String, serde_json::Value>> {
        Ok(IssueFieldValueRepo::list_by_issue(conn, issue_id)?)
    }
}
