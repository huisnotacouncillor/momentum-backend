//! Feature flag system (spec §5)
//!
//! 暂不替代任何现有分发逻辑；只是把 "command 是否启用" 这个判断单独抽出来。
//! 之后 `dispatch` 或 middleware 层会调 `flags.is_command_enabled(cmd)`。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FeatureFlags {
    // Issue
    pub issue_events_enabled: bool,
    pub issue_create_enabled: bool,
    pub issue_update_enabled: bool,
    pub issue_delete_enabled: bool,
    pub issue_query_enabled: bool,

    // Project
    pub project_events_enabled: bool,
    pub project_crud_enabled: bool,

    // Label
    pub label_events_enabled: bool,
    pub label_crud_enabled: bool,

    // Workspace
    pub workspace_events_enabled: bool,
    pub workspace_crud_enabled: bool,

    // Advanced
    pub batch_operations_enabled: bool,
    pub comments_enabled: bool,
    pub attachments_enabled: bool,

    // System
    pub subscription_enabled: bool,
    pub broadcast_enabled: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            issue_events_enabled: false,
            issue_create_enabled: false,
            issue_update_enabled: false,
            issue_delete_enabled: false,
            issue_query_enabled: false,

            project_events_enabled: false,
            project_crud_enabled: false,

            label_events_enabled: false,
            label_crud_enabled: false,

            // workspace 已稳定，默认开启（对齐前端默认值）
            workspace_events_enabled: true,
            workspace_crud_enabled: true,

            batch_operations_enabled: false,
            comments_enabled: false,
            attachments_enabled: false,

            subscription_enabled: true,
            broadcast_enabled: true,
        }
    }
}

impl FeatureFlags {
    /// 检查一个 command 是否启用
    ///
    /// 与现有 `WebSocketCommand` enum 配合时：
    /// - 大多数与 enum variant 同名（snake_case）；
    /// - 总是启用的命令（ping / get_connection_info）返回 true。
    pub fn is_command_enabled(&self, command_type: &str) -> bool {
        match command_type {
            // Issue
            "create_issue" => self.issue_create_enabled,
            "update_issue" => self.issue_update_enabled,
            "delete_issue" => self.issue_delete_enabled,
            "query_issues" | "get_issue" => self.issue_query_enabled,

            // Project
            "create_project" | "update_project" | "delete_project" | "query_projects"
            | "get_project" => self.project_crud_enabled,

            // Label
            "create_label" | "update_label" | "delete_label" | "query_labels"
            | "get_label" | "batch_create_labels" | "batch_update_labels"
            | "batch_delete_labels" => self.label_crud_enabled,

            // Workspace
            "create_workspace" | "update_workspace" | "delete_workspace"
            | "get_current_workspace" => self.workspace_crud_enabled,

            // 始终启用：连接维护命令
            "ping" | "get_connection_info" | "subscribe" | "unsubscribe"
            | "get_feature_flags" => true,

            // 未知命令默认禁用（保守）
            _ => false,
        }
    }

    /// 从环境变量加载（FF_<UPPER_SNAKE>），找不到则走 default
    pub fn from_env() -> Self {
        let mut f = Self::default();
        let set_bool = |target: &mut bool, key: &str| {
            if let Ok(v) = std::env::var(key) {
                *target = v == "true" || v == "1";
            }
        };
        set_bool(&mut f.issue_events_enabled, "FF_ISSUE_EVENTS");
        set_bool(&mut f.issue_create_enabled, "FF_ISSUE_CREATE");
        set_bool(&mut f.issue_update_enabled, "FF_ISSUE_UPDATE");
        set_bool(&mut f.issue_delete_enabled, "FF_ISSUE_DELETE");
        set_bool(&mut f.issue_query_enabled, "FF_ISSUE_QUERY");
        set_bool(&mut f.project_events_enabled, "FF_PROJECT_EVENTS");
        set_bool(&mut f.project_crud_enabled, "FF_PROJECT_CRUD");
        set_bool(&mut f.label_events_enabled, "FF_LABEL_EVENTS");
        set_bool(&mut f.label_crud_enabled, "FF_LABEL_CRUD");
        set_bool(&mut f.workspace_events_enabled, "FF_WORKSPACE_EVENTS");
        set_bool(&mut f.workspace_crud_enabled, "FF_WORKSPACE_CRUD");
        set_bool(&mut f.batch_operations_enabled, "FF_BATCH_OPERATIONS");
        set_bool(&mut f.comments_enabled, "FF_COMMENTS");
        set_bool(&mut f.attachments_enabled, "FF_ATTACHMENTS");
        set_bool(&mut f.subscription_enabled, "FF_SUBSCRIPTION");
        set_bool(&mut f.broadcast_enabled, "FF_BROADCAST");
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_disables_issue_project_label() {
        let f = FeatureFlags::default();
        assert!(!f.issue_create_enabled);
        assert!(!f.project_crud_enabled);
        assert!(!f.label_crud_enabled);
    }

    #[test]
    fn default_enables_workspace_basic() {
        let f = FeatureFlags::default();
        assert!(f.workspace_events_enabled);
        assert!(f.workspace_crud_enabled);
        assert!(f.subscription_enabled);
        assert!(f.broadcast_enabled);
    }

    #[test]
    fn is_command_enabled_workspace_works() {
        let f = FeatureFlags::default();
        assert!(f.is_command_enabled("create_workspace"));
        assert!(f.is_command_enabled("delete_workspace"));
    }

    #[test]
    fn is_command_enabled_issue_blocked_by_default() {
        let f = FeatureFlags::default();
        assert!(!f.is_command_enabled("create_issue"));
        assert!(!f.is_command_enabled("query_issues"));
    }

    #[test]
    fn toggle_then_check() {
        let mut f = FeatureFlags::default();
        assert!(!f.is_command_enabled("create_issue"));
        f.issue_create_enabled = true;
        assert!(f.is_command_enabled("create_issue"));
    }

    #[test]
    fn ping_always_enabled() {
        let f = FeatureFlags::default();
        assert!(f.is_command_enabled("ping"));
        assert!(f.is_command_enabled("get_connection_info"));
    }

    #[test]
    fn unknown_command_default_disabled() {
        let f = FeatureFlags::default();
        assert!(!f.is_command_enabled("not_a_real_command"));
    }
}
