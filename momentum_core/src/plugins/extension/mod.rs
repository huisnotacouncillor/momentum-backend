//! 8 大扩展点的 v0.1 实现
//!
//! v0.1: Field / Agent / Storage / Event（完整）
//! v0.2: View / Workflow / Integration（stub）
//! v0.3: 自定义视图 + Marketplace
//!
//! 详见 docs/PLUGIN_SDK_DESIGN.md §3

pub mod agent;
pub mod event;
pub mod field;
pub mod storage;

pub use agent::AgentService;
pub use event::EventService;
pub use field::FieldService;
pub use storage::StorageService;
