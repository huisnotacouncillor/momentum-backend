//! Plugin Manifest 解析与验证
//!
//! Manifest 格式：plugin.yaml
//! 详见 docs/PLUGIN_SDK_DESIGN.md §2

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::error::{PluginError, PluginResult};

/// Manifest schema 版本
pub const MANIFEST_API_VERSION: &str = "v1";

/// 插件清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "apiVersion")]
    pub api_version: String,

    #[serde(default)]
    pub kind: ManifestKind,

    // === 标识 ===
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,

    // === 兼容 ===
    #[serde(rename = "core_compat", default)]
    pub core_compat: Option<String>,

    // === 入口 ===
    #[serde(default)]
    pub entrypoint: Option<Entrypoint>,

    // === 扩展点 ===
    #[serde(default)]
    pub extensions: Extensions,

    // === 权限申请 ===
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum ManifestKind {
    #[default]
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entrypoint {
    #[serde(default)]
    pub binary: Option<String>,
    #[serde(default)]
    pub container: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extensions {
    #[serde(default)]
    pub fields: Vec<FieldDef>,

    #[serde(default, rename = "artifact_types")]
    pub artifact_types: Vec<String>,

    #[serde(default)]
    pub workflows: Vec<WorkflowDef>,

    #[serde(default)]
    pub agents: Vec<AgentDef>,

    #[serde(default)]
    pub views: Vec<ViewDef>,

    #[serde(default)]
    pub integrations: Vec<IntegrationDef>,

    #[serde(default)]
    pub webhooks: WebhookDef,

    #[serde(default)]
    pub storage: Vec<StorageDef>,
}

// === 8 大扩展点定义 ===

/// 扩展点 1：字段扩展
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// 完整 key，如 "issue.effort" / "issue.safety_level"
    pub key: String,

    /// 字段类型
    #[serde(rename = "type")]
    pub field_type: FieldType,

    /// 显示标签
    pub label: String,

    #[serde(default)]
    pub required: bool,

    #[serde(default)]
    pub options: Option<serde_json::Value>,

    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
    Enum,
    Date,
    User,
    Bool,
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::Number => "number",
            FieldType::Enum => "enum",
            FieldType::Date => "date",
            FieldType::User => "user",
            FieldType::Bool => "bool",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(FieldType::Text),
            "number" => Some(FieldType::Number),
            "enum" => Some(FieldType::Enum),
            "date" => Some(FieldType::Date),
            "user" => Some(FieldType::User),
            "bool" => Some(FieldType::Bool),
            _ => None,
        }
    }
}

/// 扩展点 3：Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
}

/// 扩展点 4：Workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDef {
    pub id: String,
    pub trigger: String,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub steps: Vec<serde_json::Value>,
}

/// 扩展点 2：View
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewDef {
    pub id: String,
    pub label: String,
    pub slot: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
}

/// 扩展点 5：Integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationDef {
    #[serde(rename = "type")]
    pub integration_type: String,
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,
}

/// 扩展点 6：Webhook
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebhookDef {
    #[serde(default)]
    pub subscribes: Vec<String>,
    #[serde(default)]
    pub publishes: Vec<String>,
}

/// 扩展点 7：Storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDef {
    pub namespace: String,
    #[serde(default = "default_storage_size")]
    pub max_size_mb: i64,
}

fn default_storage_size() -> i64 {
    100
}

/// 解析并验证 Manifest
pub fn parse_manifest(yaml: &str) -> PluginResult<Manifest> {
    let manifest: Manifest = serde_yaml::from_str(yaml)?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// 从文件加载 Manifest
pub fn load_manifest_from_file<P: AsRef<Path>>(path: P) -> PluginResult<Manifest> {
    let content = std::fs::read_to_string(path.as_ref())
        .map_err(|e| PluginError::ManifestInvalid(format!("read failed: {}", e)))?;
    parse_manifest(&content)
}

/// 验证 Manifest 合法性
pub fn validate_manifest(m: &Manifest) -> PluginResult<()> {
    // 1. apiVersion
    if m.api_version != MANIFEST_API_VERSION {
        return Err(PluginError::ManifestInvalid(format!(
            "unsupported apiVersion: {} (expected {})",
            m.api_version, MANIFEST_API_VERSION
        )));
    }

    // 2. kind
    if m.kind != ManifestKind::Plugin {
        return Err(PluginError::ManifestInvalid(format!(
            "unsupported kind: {:?}",
            m.kind
        )));
    }

    // 3. id: 必须反向 DNS 风格
    if !is_valid_plugin_id(&m.id) {
        return Err(PluginError::ManifestInvalid(format!(
            "invalid plugin id '{}': must be reverse-DNS like 'embodied-intelligence'",
            m.id
        )));
    }

    // 4. version: 必须是 semver
    if semver::Version::parse(&m.version).is_err() {
        return Err(PluginError::ManifestInvalid(format!(
            "invalid version '{}': must be semver (e.g. 1.0.0)",
            m.version
        )));
    }

    // 5. 字段 key 不能重复
    let mut seen_fields = HashMap::new();
    for f in &m.extensions.fields {
        if seen_fields.insert(f.key.clone(), ()).is_some() {
            return Err(PluginError::ManifestInvalid(format!(
                "duplicate field key: {}",
                f.key
            )));
        }
        if !f.key.starts_with("issue.") {
            return Err(PluginError::ManifestInvalid(format!(
                "field key must start with 'issue.': {}",
                f.key
            )));
        }
    }

    // 6. agent id 不能重复
    let mut seen_agents = HashMap::new();
    for a in &m.extensions.agents {
        if seen_agents.insert(a.id.clone(), ()).is_some() {
            return Err(PluginError::ManifestInvalid(format!(
                "duplicate agent id: {}",
                a.id
            )));
        }
    }

    // 7. 权限列表格式校验
    for p in &m.permissions {
        if !is_valid_permission(p) {
            return Err(PluginError::ManifestInvalid(format!(
                "invalid permission format: {}",
                p
            )));
        }
    }

    // 8. event publish/subscribe 必须在 permissions 申请
    for evt in &m.extensions.webhooks.publishes {
        let perm = format!("event.publish:{}", evt);
        if !m.permissions.contains(&perm) {
            return Err(PluginError::ManifestInvalid(format!(
                "plugin publishes event '{}' but not in permissions (add '{}')",
                evt, perm
            )));
        }
    }
    for evt in &m.extensions.webhooks.subscribes {
        let perm = format!("event.subscribe:{}", evt);
        if !m.permissions.contains(&perm) {
            return Err(PluginError::ManifestInvalid(format!(
                "plugin subscribes event '{}' but not in permissions (add '{}')",
                evt, perm
            )));
        }
    }

    Ok(())
}

fn is_valid_plugin_id(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 {
        return false;
    }
    let chars: Vec<char> = s.chars().collect();
    if !chars.iter().all(|c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '_'
    }) {
        return false;
    }
    if chars[0] == '.' || chars[0] == '-' {
        return false;
    }
    if *chars.last().unwrap() == '.' || *chars.last().unwrap() == '-' {
        return false;
    }
    true
}

/// 验证权限字符串格式: "domain.action[:resource]"
fn is_valid_permission(s: &str) -> bool {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.is_empty() {
        return false;
    }
    let domain_action: Vec<&str> = parts[0].split('.').collect();
    if domain_action.len() < 2 {
        return false;
    }
    for p in &domain_action {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            return false;
        }
    }
    if parts.len() == 2 && parts[1].is_empty() {
        return false;
    }
    true
}

// =============================================================
// 单元测试
// =============================================================

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_YAML: &str = r#"
apiVersion: v1
kind: Plugin
id: dummy-plugin
name: Dummy Plugin
version: 0.1.0
publisher: Test
core_compat: ">=1.0.0"
entrypoint:
  binary: ./bin/dummy
extensions:
  fields:
    - key: issue.effort
      type: number
      label: Effort
  agents:
    - id: dummy-agent
      description: Test agent
  webhooks:
    subscribes: [issue.created]
    publishes: [dummy.test]
  storage:
    - namespace: telemetry
permissions:
  - issue.read
  - issue.write
  - issue.field.write:issue.effort
  - agent.invoke:dummy-agent
  - event.subscribe:issue.created
  - event.publish:dummy.test
  - storage.write:telemetry
"#;

    #[test]
    fn test_parse_valid_manifest() {
        let m = parse_manifest(VALID_YAML).unwrap();
        assert_eq!(m.id, "dummy-plugin");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.extensions.fields.len(), 1);
        assert_eq!(m.extensions.fields[0].field_type, FieldType::Number);
        assert_eq!(m.extensions.agents[0].id, "dummy-agent");
    }

    #[test]
    fn test_invalid_api_version() {
        let yaml = r#"
apiVersion: v2
kind: Plugin
id: foo
name: Foo
version: 1.0.0
"#;
        assert!(parse_manifest(yaml).is_err());
    }

    #[test]
    fn test_invalid_plugin_id() {
        let yaml = r#"
apiVersion: v1
kind: Plugin
id: "Foo_Bar"
name: Foo
version: 1.0.0
"#;
        assert!(parse_manifest(yaml).is_err());
    }

    #[test]
    fn test_invalid_version() {
        let yaml = r#"
apiVersion: v1
kind: Plugin
id: foo
name: Foo
version: "not-semver"
"#;
        assert!(parse_manifest(yaml).is_err());
    }

    #[test]
    fn test_field_key_must_start_with_issue() {
        let yaml = r#"
apiVersion: v1
kind: Plugin
id: foo
name: Foo
version: 1.0.0
extensions:
  fields:
    - key: project.status
      type: text
      label: Status
"#;
        assert!(parse_manifest(yaml).is_err());
    }

    #[test]
    fn test_duplicate_field_key() {
        let yaml = r#"
apiVersion: v1
kind: Plugin
id: foo
name: Foo
version: 1.0.0
extensions:
  fields:
    - key: issue.foo
      type: text
      label: Foo
    - key: issue.foo
      type: number
      label: Foo
"#;
        assert!(parse_manifest(yaml).is_err());
    }

    #[test]
    fn test_publish_event_requires_permission() {
        let yaml = r#"
apiVersion: v1
kind: Plugin
id: foo
name: Foo
version: 1.0.0
extensions:
  webhooks:
    publishes: [foo.bar]
permissions:
  - issue.read
"#;
        assert!(parse_manifest(yaml).is_err());
    }

    #[test]
    fn test_is_valid_plugin_id() {
        assert!(is_valid_plugin_id("dummy-plugin"));
        assert!(is_valid_plugin_id("embodied-intelligence"));
        assert!(is_valid_plugin_id("com.acme.foo"));
        assert!(!is_valid_plugin_id("Foo"));
        assert!(!is_valid_plugin_id("-foo"));
        assert!(!is_valid_plugin_id("foo-"));
        assert!(!is_valid_plugin_id(""));
        assert!(!is_valid_plugin_id(".foo"));
        assert!(!is_valid_plugin_id("foo."));
    }

    #[test]
    fn test_is_valid_permission() {
        assert!(is_valid_permission("issue.read"));
        assert!(is_valid_permission("issue.field.write:issue.effort"));
        assert!(is_valid_permission("agent.invoke:spec-agent"));
        assert!(!is_valid_permission("issue"));
        assert!(!is_valid_permission("issue.Read"));
        assert!(!is_valid_permission("issue.read:"));
    }
}
