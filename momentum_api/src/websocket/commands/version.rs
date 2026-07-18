//! Command versioning management
//!
//! Design: 命令级版本化（Command-level versioning）
//! - 每个命令独立版本，不强制连接升级
//! - v1 命令保持兼容，v2 命令使用新结构
//! - 当 client 使用旧版本时，响应中标记 deprecated
//!
//! ## 版本策略
//!
//! - `create_issue` v1: { title, description, ... }
//! - `create_issue` v2: { title, description, body, ... }  // 未来
//!
//! Client 可以在 command payload 中指定 version 字段（可选），不指定默认 v1

/// 返回命令的最新版本号
///
/// # Example
///
/// ```
/// assert_eq!(command_latest_version("create_issue"), 1);
/// ```
pub fn command_latest_version(command_type: &str) -> u32 {
    match command_type {
        // ============================================================
        // V2 Commands (未来添加)
        // ============================================================
        // 当某个命令需要破坏性变更时，创建 v2 版本
        // 旧版本保持可用，但响应中标记 deprecated
        //
        // "create_issue" => 2,  // v1 -> v2
        // "update_issue" => 2,
        // "create_team"  => 2,

        // ============================================================
        // V1 Commands (当前默认)
        // ============================================================
        _ => 1,
    }
}

/// 检查命令是否已废弃
///
/// 当 client 使用的版本 < 最新版本时，返回 true
pub fn is_command_deprecated(command_type: &str, client_version: u32) -> bool {
    command_latest_version(command_type) > client_version
}

/// 获取命令版本信息
#[derive(Debug, Clone)]
pub struct CommandVersionInfo {
    /// 命令实际执行的版本
    pub version: u32,
    /// 命令的最新版本
    pub latest_version: u32,
    /// 是否已废弃
    pub deprecated: bool,
}

impl Default for CommandVersionInfo {
    fn default() -> Self {
        Self {
            version: 1,
            latest_version: 1,
            deprecated: false,
        }
    }
}

impl CommandVersionInfo {
    /// 从 command_type 和 client 指定的版本创建
    pub fn new(command_type: &str, client_version: Option<u32>) -> Self {
        let latest = command_latest_version(command_type);
        let version = client_version.unwrap_or(1);

        Self {
            version,
            latest_version: latest,
            deprecated: latest > version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_commands_default_to_v1() {
        assert_eq!(command_latest_version("create_issue"), 1);
        assert_eq!(command_latest_version("update_issue"), 1);
        assert_eq!(command_latest_version("unknown_command"), 1);
    }

    #[test]
    fn test_no_commands_are_deprecated_when_all_v1() {
        assert!(!is_command_deprecated("create_issue", 1));
        // version 0 < latest(1) = deprecated
        assert!(is_command_deprecated("create_issue", 0));
    }

    #[test]
    fn test_command_version_info() {
        let info = CommandVersionInfo::new("create_issue", Some(1));
        assert_eq!(info.version, 1);
        assert_eq!(info.latest_version, 1);
        assert!(!info.deprecated);

        // 默认版本
        let info_default = CommandVersionInfo::new("create_issue", None);
        assert_eq!(info_default.version, 1);
    }
}
