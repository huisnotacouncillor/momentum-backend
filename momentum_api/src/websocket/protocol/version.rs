//! Protocol version (spec §7)
//!
//! 仅定义 version 数据结构 + 协商函数；中间件放在 Step 4 加。
//! 当前 server 一律支持 V1_0 / V1_1。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const V1_0: Self = Self { major: 1, minor: 0 };
    pub const V1_1: Self = Self { major: 1, minor: 1 };

    pub fn parse(s: &str) -> Option<Self> {
        let (a, b) = s.split_once('.')?;
        let major = a.parse().ok()?;
        let minor = b.parse().ok()?;
        Some(Self { major, minor })
    }

    pub fn as_string(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }

    /// 主版本号相同即可互操作（minor 向下兼容）
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major
    }

    pub fn supported() -> &'static [ProtocolVersion] {
        &[Self::V1_0, Self::V1_1]
    }

    pub fn latest() -> Self {
        Self::V1_1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionNegotiation {
    pub client_version: ProtocolVersion,
    pub server_version: ProtocolVersion,
    pub agreed_version: ProtocolVersion,
    pub is_compatible: bool,
}

impl VersionNegotiation {
    /// 协商规则：
    /// 1. 主版本不同 -> is_compatible=false；server 强切到 latest；
    /// 2. 主版本相同 -> 取较小一方以保证双向兼容；
    pub fn negotiate(client: ProtocolVersion, server: ProtocolVersion) -> Self {
        let is_compatible = client.is_compatible_with(&server);
        let agreed = if is_compatible {
            // minor 取小
            if client <= server { client } else { server }
        } else {
            ProtocolVersion::latest()
        };
        Self {
            client_version: client,
            server_version: server,
            agreed_version: agreed,
            is_compatible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        assert_eq!(ProtocolVersion::parse("1.0"), Some(ProtocolVersion::V1_0));
        assert_eq!(ProtocolVersion::parse("1.1"), Some(ProtocolVersion::V1_1));
        assert_eq!(ProtocolVersion::parse("2.0"), Some(ProtocolVersion { major: 2, minor: 0 }));
        assert_eq!(ProtocolVersion::parse("not.a.version"), None);
        assert_eq!(ProtocolVersion::parse("1"), None);
    }

    #[test]
    fn compatibility_same_major() {
        assert!(ProtocolVersion::V1_0.is_compatible_with(&ProtocolVersion::V1_1));
        assert!(!ProtocolVersion::V1_0.is_compatible_with(&ProtocolVersion { major: 2, minor: 0 }));
    }

    #[test]
    fn negotiate_same_major_picks_minor_min() {
        let n = VersionNegotiation::negotiate(
            ProtocolVersion::V1_0,
            ProtocolVersion::V1_1,
        );
        assert!(n.is_compatible);
        assert_eq!(n.agreed_version, ProtocolVersion::V1_0);
    }

    #[test]
    fn negotiate_compatible_same_version() {
        let n = VersionNegotiation::negotiate(
            ProtocolVersion::V1_1,
            ProtocolVersion::V1_1,
        );
        assert!(n.is_compatible);
        assert_eq!(n.agreed_version, ProtocolVersion::V1_1);
    }

    #[test]
    fn negotiate_incompatible_falls_back_to_latest() {
        let n = VersionNegotiation::negotiate(
            ProtocolVersion::V1_0,
            ProtocolVersion { major: 2, minor: 5 },
        );
        assert!(!n.is_compatible);
        assert_eq!(n.agreed_version, ProtocolVersion::latest());
    }

    #[test]
    fn as_string_roundtrip() {
        let v = ProtocolVersion::parse("1.1").unwrap();
        assert_eq!(v.as_string(), "1.1");
    }
}
