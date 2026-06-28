//! Plugin 权限检查
//!
//! 详见 docs/PLUGIN_SDK_DESIGN.md §8

use super::error::{PluginError, PluginResult};
use super::manifest::Manifest;

/// 权限字符串
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub domain: String,
    pub action: String,
    pub resource: Option<String>,
}

impl Permission {
    pub fn parse(s: &str) -> PluginResult<Self> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        let domain_action: Vec<&str> = parts[0].split('.').collect();
        if domain_action.len() < 2 {
            return Err(PluginError::PermissionDenied(format!(
                "invalid permission format: {}",
                s
            )));
        }
        let domain = domain_action[0].to_string();
        let action = domain_action[1..].join(".");
        let resource = if parts.len() == 2 {
            let r = parts[1];
            if r.is_empty() {
                return Err(PluginError::PermissionDenied(format!(
                    "invalid permission format (empty resource): {}",
                    s
                )));
            }
            Some(r.to_string())
        } else {
            None
        };
        Ok(Permission {
            domain,
            action,
            resource,
        })
    }

    /// 是否匹配另一个权限（被 grant 的权限是否覆盖 requested）
    pub fn matches(&self, granted: &Permission) -> bool {
        if self.domain != granted.domain {
            return false;
        }
        if self.action != granted.action {
            return false;
        }
        match (&self.resource, &granted.resource) {
            (None, _) => true,
            (Some(req), Some(grant)) => req == grant,
            (Some(_), None) => false,
        }
    }
}

/// 检查插件是否被授予某权限
pub fn check_permission(manifest: &Manifest, requested: &str) -> PluginResult<()> {
    let requested_p = Permission::parse(requested)?;

    for granted_str in &manifest.permissions {
        let granted_p = Permission::parse(granted_str)?;
        if requested_p.matches(&granted_p) {
            return Ok(());
        }
    }

    Err(PluginError::PermissionDenied(format!(
        "permission '{}' not granted to plugin {}",
        requested, manifest.id
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::parse_manifest;

    #[test]
    fn test_parse_permission() {
        let p = Permission::parse("issue.read").unwrap();
        assert_eq!(p.domain, "issue");
        assert_eq!(p.action, "read");
        assert_eq!(p.resource, None);

        let p = Permission::parse("issue.field.write:issue.effort").unwrap();
        assert_eq!(p.domain, "issue");
        assert_eq!(p.action, "field.write");
        assert_eq!(p.resource, Some("issue.effort".to_string()));
    }

    #[test]
    fn test_parse_rejects_empty_resource() {
        assert!(Permission::parse("issue.read:").is_err());
    }

    #[test]
    fn test_matches() {
        let granted = Permission::parse("issue.field.write:issue.effort").unwrap();
        let requested = Permission::parse("issue.field.write:issue.effort").unwrap();
        assert!(requested.matches(&granted));

        let granted = Permission::parse("issue.field.write:issue.effort").unwrap();
        let requested = Permission::parse("issue.field.write:issue.priority").unwrap();
        assert!(!requested.matches(&granted));

        let granted = Permission::parse("issue.read").unwrap();
        let requested = Permission::parse("agent.read").unwrap();
        assert!(!requested.matches(&granted));

        let granted = Permission::parse("issue.read").unwrap();
        let requested = Permission::parse("issue.write").unwrap();
        assert!(!requested.matches(&granted));
    }

    #[test]
    fn test_check_permission() {
        let yaml = r#"
apiVersion: v1
kind: Plugin
id: dummy
name: D
version: 1.0.0
permissions:
  - issue.read
  - issue.field.write:issue.effort
  - agent.invoke:dummy-agent
"#;
        let m = parse_manifest(yaml).unwrap();

        assert!(check_permission(&m, "issue.read").is_ok());
        assert!(check_permission(&m, "issue.field.write:issue.effort").is_ok());

        assert!(check_permission(&m, "issue.write").is_err());
        assert!(check_permission(&m, "issue.field.write:issue.priority").is_err());
        assert!(check_permission(&m, "agent.invoke:other-agent").is_err());
    }
}
