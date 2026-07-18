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
mod topic_tests {
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

//! SubscriptionManager (spec §6.2)
//!
//! 服务端 fanout 注册表：
//!   topic_key -> [connection_id]
//!   connection_id -> [topic_key]
//!
//! 设计原则：
//! - **不假设**任何具体 connection 类型；只用 `connection_id: String` 作为键
//!   避免与 `WebSocketManager` 强耦合（Step 8 之前不接 manager，单独可测）
//! - **不阻塞**：所有读写都走 `tokio::sync::RwLock`
//! - **匹配**对外暴露 `subscribers_of(event: &Topic) -> Vec<String>`，fanout
//!   端只需遍历此结果

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeResult {
    pub subscribed: Vec<Topic>,
    pub duplicates: Vec<Topic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeResult {
    pub unsubscribed: Vec<Topic>,
}

#[derive(Default)]
pub struct SubscriptionManager {
    /// topic -> connection_ids
    topic_subs: RwLock<HashMap<Topic, HashSet<String>>>,
    /// connection_id -> topics（便于 unsubscribe_all/leave）
    conn_topics: RwLock<HashMap<String, HashSet<Topic>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 订阅。
    /// 返回 `SubscribeResult{ subscribed, duplicates }`：已经订阅过的 topic
    /// 在 `duplicates` 中，不重复加入。
    pub async fn subscribe(
        &self,
        connection_id: &str,
        topics: &[Topic],
    ) -> SubscribeResult {
        let mut t_sub = self.topic_subs.write().await;
        let mut c_sub = self.conn_topics.write().await;

        let conn = c_sub
            .entry(connection_id.to_string())
            .or_default();

        let mut subscribed = Vec::new();
        let mut duplicates = Vec::new();

        for t in topics {
            if conn.contains(t) {
                duplicates.push(t.clone());
                continue;
            }
            conn.insert(t.clone());
            t_sub.entry(t.clone()).or_default().insert(connection_id.to_string());
            subscribed.push(t.clone());
        }
        SubscribeResult { subscribed, duplicates }
    }

    /// 取消订阅
    pub async fn unsubscribe(
        &self,
        connection_id: &str,
        topics: &[Topic],
    ) -> UnsubscribeResult {
        let mut t_sub = self.topic_subs.write().await;
        let mut c_sub = self.conn_topics.write().await;

        let mut unsubscribed = Vec::new();
        for t in topics {
            // 从 topic_subs 中移除
            let mut removed = false;
            if let Some(set) = t_sub.get_mut(t) {
                removed = set.remove(connection_id);
                if set.is_empty() {
                    t_sub.remove(t);
                }
            }
            // 从 conn_topics 中移除
            if let Some(set) = c_sub.get_mut(connection_id) {
                set.remove(t);
            }
            if removed {
                unsubscribed.push(t.clone());
            }
        }
        UnsubscribeResult { unsubscribed }
    }

    /// 断开连接时清空其所有订阅
    pub async fn leave(&self, connection_id: &str) {
        let mut t_sub = self.topic_subs.write().await;
        let mut c_sub = self.conn_topics.write().await;

        if let Some(topics) = c_sub.remove(connection_id) {
            for t in topics {
                if let Some(set) = t_sub.get_mut(&t) {
                    set.remove(connection_id);
                    if set.is_empty() {
                        t_sub.remove(&t);
                    }
                }
            }
        }
    }

    /// 拿到应被 fan-out 给定的 event 的所有 connection_ids
    /// 匹配规则由 `Topic::matches` 给出
    pub async fn subscribers_of(&self, event: &Topic) -> Vec<String> {
        let t_sub = self.topic_subs.read().await;
        let mut hits = HashSet::new();
        for (sub_topic, conns) in t_sub.iter() {
            if sub_topic.matches(event) {
                for c in conns {
                    hits.insert(c.clone());
                }
            }
        }
        hits.into_iter().collect()
    }

    /// 当前活跃连接数
    pub async fn connection_count(&self) -> usize {
        self.conn_topics.read().await.len()
    }

    /// 当前主题数量
    pub async fn topic_count(&self) -> usize {
        self.topic_subs.read().await.len()
    }

    /// 给定 connection 当前的订阅列表
    pub async fn subscriptions_of(&self, connection_id: &str) -> Vec<Topic> {
        self.conn_topics
            .read()
            .await
            .get(connection_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_mgr() -> SubscriptionManager {
        SubscriptionManager::new()
    }

    #[tokio::test]
    async fn subscribe_then_subscribers_of() {
        let mgr = make_mgr().await;
        let t = Topic::parse("issues").unwrap();
        let r = mgr.subscribe("c1", &[t.clone()]).await;
        assert_eq!(r.subscribed.len(), 1);
        assert!(r.duplicates.is_empty());

        let hits = mgr.subscribers_of(&Topic::parse("issues:abc:created").unwrap()).await;
        let mut sorted = hits;
        sorted.sort();
        assert_eq!(sorted, vec!["c1".to_string()]);
    }

    #[tokio::test]
    async fn duplicate_subscribe_marks_duplicate() {
        let mgr = make_mgr().await;
        let t = Topic::parse("issues").unwrap();
        let r1 = mgr.subscribe("c1", &[t.clone()]).await;
        let r2 = mgr.subscribe("c1", &[t.clone()]).await;
        assert_eq!(r1.subscribed.len(), 1);
        assert!(r2.subscribed.is_empty());
        assert_eq!(r2.duplicates.len(), 1);
    }

    #[tokio::test]
    async fn multiple_connections_match_wildcard() {
        let mgr = make_mgr().await;
        let t = Topic::parse("projects:*:created").unwrap();
        mgr.subscribe("c1", &[t.clone()]).await;
        mgr.subscribe("c2", &[t.clone()]).await;

        let hits = mgr
            .subscribers_of(&Topic::parse("projects:pid:created").unwrap())
            .await;
        let mut sorted = hits;
        sorted.sort();
        assert_eq!(sorted, vec!["c1".to_string(), "c2".to_string()]);
    }

    #[tokio::test]
    async fn unsubscribe_single() {
        let mgr = make_mgr().await;
        let t1 = Topic::parse("issues").unwrap();
        let t2 = Topic::parse("projects").unwrap();
        mgr.subscribe("c1", &[t1.clone(), t2.clone()]).await;
        let r = mgr.unsubscribe("c1", &[t1.clone()]).await;
        assert_eq!(r.unsubscribed.len(), 1);
        let hits = mgr.subscribers_of(&Topic::parse("issues").unwrap()).await;
        assert!(hits.is_empty());
        let hits = mgr.subscribers_of(&Topic::parse("projects").unwrap()).await;
        assert_eq!(hits, vec!["c1".to_string()]);
    }

    #[tokio::test]
    async fn leave_clears_all_subscriptions() {
        let mgr = make_mgr().await;
        mgr.subscribe("c1", &[Topic::parse("issues").unwrap()]).await;
        mgr.subscribe("c1", &[Topic::parse("projects").unwrap()]).await;
        mgr.leave("c1").await;
        assert_eq!(mgr.connection_count().await, 0);
        assert_eq!(mgr.topic_count().await, 0);
    }

    #[tokio::test]
    async fn leave_keeps_other_connections() {
        let mgr = make_mgr().await;
        let t = Topic::parse("issues").unwrap();
        mgr.subscribe("c1", &[t.clone()]).await;
        mgr.subscribe("c2", &[t.clone()]).await;
        mgr.leave("c1").await;
        let hits = mgr.subscribers_of(&Topic::parse("issues").unwrap()).await;
        assert_eq!(hits, vec!["c2".to_string()]);
    }

    #[tokio::test]
    async fn subscribers_of_returns_empty_for_no_match() {
        let mgr = make_mgr().await;
        mgr.subscribe("c1", &[Topic::parse("issues").unwrap()]).await;
        let hits = mgr.subscribers_of(&Topic::parse("projects").unwrap()).await;
        assert!(hits.is_empty());
    }

    #[tokio::test]
    async fn subscriptions_of_lists_topics() {
        let mgr = make_mgr().await;
        mgr.subscribe(
            "c1",
            &[
                Topic::parse("issues").unwrap(),
                Topic::parse("projects:abc").unwrap(),
            ],
        )
        .await;
        let subs = mgr.subscriptions_of("c1").await;
        assert_eq!(subs.len(), 2);
    }
}
