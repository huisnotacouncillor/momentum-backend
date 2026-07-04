//! Subscription module 索引
//!
//! 不依赖 WebSocketManager；先用 connection_id (String) 作为键，
//! 后续 Step 8+ 在 manager.rs 中接入真实连接句柄。

pub mod manager;
pub mod topic;

pub use manager::{SubscribeResult, SubscriptionManager, UnsubscribeResult};
pub use topic::{Topic, TopicParseError};
