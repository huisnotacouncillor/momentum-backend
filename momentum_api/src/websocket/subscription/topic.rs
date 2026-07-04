//! Topic parse / match (spec §6.1)
//!
//! 协议格式：`namespace[:resource_id[:action]]`
//! - resource_id 为 `*` 或缺省 = 通配
//! - action 为 `*` 或缺省 = 通配
//!
//! 示例：
//! - `"issues"`                  -> 任何 issue 事件
//! - `"issues:*"`                -> 同上
//! - `"issues:*:created"`        -> 任何 issue 创建
//! - `"issues:abc-uuid"`         -> 单条 issue
//! - `"issues:abc-uuid:created"` -> 单条 issue 的 created 事件
//!
//! 解析时 resource_id **不**强校验为 UUID；字符串相等即算匹配，
//! 因为旧事件可能用字符串形式（如 issue_key）。上层决定是否做转换。

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Topic {
    pub namespace: String,
    pub resource_id: Option<String>, // None 表示通配
    pub action: Option<String>,      // None 表示通配
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TopicParseError {
    #[error("topic is empty")]
    Empty,
    #[error("topic namespace is empty: {0}")]
    EmptyNamespace(String),
    #[error("invalid characters in segment")]
    InvalidCharacters,
}

impl Topic {
    /// 解析 topic 字符串
    pub fn parse(s: &str) -> Result<Self, TopicParseError> {
        if s.is_empty() {
            return Err(TopicParseError::Empty);
        }
        let parts: Vec<&str> = s.split(':').collect();
        let namespace = parts[0];
        if namespace.is_empty() {
            return Err(TopicParseError::EmptyNamespace(s.to_string()));
        }
        if !is_valid_segment(namespace) {
            return Err(TopicParseError::InvalidCharacters);
        }

        let resource_id = match parts.get(1).copied() {
            None => None,
            Some("*") | Some("") => None,
            Some(other) => {
                if !is_valid_segment(other) {
                    return Err(TopicParseError::InvalidCharacters);
                }
                Some(other.to_string())
            }
        };

        let action = match parts.get(2).copied() {
            None => None,
            Some("*") | Some("") => None,
            Some(other) => {
                if !is_valid_segment(other) {
                    return Err(TopicParseError::InvalidCharacters);
                }
                Some(other.to_string())
            }
        };

        Ok(Self {
            namespace: namespace.to_string(),
            resource_id,
            action,
        })
    }

    /// 序列化为字符串
    pub fn as_string(&self) -> String {
        let r = self.resource_id.as_deref().unwrap_or("*");
        let a = self.action.as_deref().unwrap_or("*");
        if self.resource_id.is_none() && self.action.is_none() {
            self.namespace.clone()
        } else if self.action.is_none() {
            format!("{}:{}", self.namespace, r)
        } else {
            format!("{}:{}:{}", self.namespace, r, a)
        }
    }

    /// 订阅匹配事件：self 是订阅，event 是事件
    /// 规则：namespace 必须相等；resource_id 若订阅具体值则事件必须相同；等等。
    pub fn matches(&self, event: &Self) -> bool {
        if self.namespace != event.namespace {
            return false;
        }
        match (&self.resource_id, &event.resource_id) {
            (Some(a), Some(b)) if a != b => return false,
            (Some(_), None) => return false, // 订阅了具体 id 但事件是通配
            _ => {}
        }
        match (&self.action, &event.action) {
            (Some(a), Some(b)) if a != b => return false,
            (Some(_), None) => return false,
            _ => {}
        }
        true
    }
}

impl fmt::Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_string())
    }
}

fn is_valid_segment(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_namespace_only() {
        let t = Topic::parse("issues").unwrap();
        assert_eq!(t.namespace, "issues");
        assert_eq!(t.resource_id, None);
        assert_eq!(t.action, None);
    }

    #[test]
    fn parses_with_wildcards() {
        let t = Topic::parse("issues:*").unwrap();
        assert!(t.resource_id.is_none());
        let t = Topic::parse("issues:*:*").unwrap();
        assert!(t.resource_id.is_none() && t.action.is_none());
    }

    #[test]
    fn parses_concrete_ids() {
        let t = Topic::parse("issues:abc-123:created").unwrap();
        assert_eq!(t.resource_id.as_deref(), Some("abc-123"));
        assert_eq!(t.action.as_deref(), Some("created"));
    }

    #[test]
    fn parses_underscored_underscore_segments() {
        let t = Topic::parse("workspace_members:user_id_42").unwrap();
        assert_eq!(t.namespace, "workspace_members");
        assert_eq!(t.resource_id.as_deref(), Some("user_id_42"));
    }

    #[test]
    fn rejects_empty() {
        assert!(Topic::parse("").is_err());
    }

    #[test]
    fn rejects_empty_namespace() {
        assert!(Topic::parse(":abc").is_err());
    }

    #[test]
    fn rejects_invalid_chars() {
        assert!(Topic::parse("issues:abc def").is_err());
    }

    #[test]
    fn matches_namespace() {
        let sub = Topic::parse("issues").unwrap();
        let ev = Topic::parse("issues:abc").unwrap();
        let ev_other = Topic::parse("projects").unwrap();
        assert!(sub.matches(&ev));
        assert!(!sub.matches(&ev_other));
    }

    #[test]
    fn matches_specific_id_mismatches_other() {
        let sub = Topic::parse("issues:abc").unwrap();
        let ev_match = Topic::parse("issues:abc:created").unwrap();
        let ev_diff = Topic::parse("issues:def:created").unwrap();
        assert!(sub.matches(&ev_match));
        assert!(!sub.matches(&ev_diff));
    }

    #[test]
    fn subscription_specific_id_does_not_match_namespace_wildcard_event() {
        let sub = Topic::parse("issues:abc").unwrap();
        let ev = Topic::parse("issues:*:created").unwrap();
        assert!(!sub.matches(&ev));
    }

    #[test]
    fn action_filter() {
        let sub = Topic::parse("issues:*:created").unwrap();
        let ev_yes = Topic::parse("issues:abc:created").unwrap();
        let ev_no = Topic::parse("issues:abc:updated").unwrap();
        assert!(sub.matches(&ev_yes));
        assert!(!sub.matches(&ev_no));
    }

    #[test]
    fn stringify_roundtrip() {
        // parse 把 "issues:*" / "issues:*:*" 视为通配 None，序列化不还原 *
        let cases = [
            ("issues", "issues"),
            ("issues:abc", "issues:abc"),
            ("issues:abc:created", "issues:abc:created"),
        ];
        for (input, expected) in cases {
            let t = Topic::parse(input).unwrap();
            assert_eq!(t.as_string(), expected, "input: {input}");
        }
    }
}
