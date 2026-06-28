//! Momentum Plugin System
//!
//! 8 大扩展点 + Manifest 驱动 + 权限校验。
//! 详见 docs/PLUGIN_SDK_DESIGN.md

pub mod audit;
pub mod error;
pub mod extension;
pub mod json_proto;
pub mod manifest;
pub mod permission;
pub mod registry;

pub use error::PluginError;
pub use extension::{AgentService, EventService, FieldService, StorageService};
pub use manifest::{AgentDef, FieldDef, Manifest, StorageDef, WebhookDef};
pub use permission::{Permission, check_permission};
