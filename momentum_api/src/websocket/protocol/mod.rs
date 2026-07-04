//! Protocol module 索引
//!
//! - `version`：ProtocolVersion / VersionNegotiation
//! - `middleware`：VersionNegotiationMiddleware

pub mod middleware;
pub mod version;

pub use middleware::VersionNegotiationMiddleware;
pub use version::{ProtocolVersion, VersionNegotiation};
