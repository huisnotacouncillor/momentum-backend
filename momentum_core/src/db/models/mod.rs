// Sub-modules organized by functional domain
pub mod agent_run;
pub mod api;
pub mod auth;
pub mod comment;
pub mod cycle;
pub mod invitation;
pub mod issue;
pub mod issue_field_definition;
pub mod issue_field_value;
pub mod label;
pub mod notification;
pub mod plugin;
pub mod plugin_audit;
pub mod plugin_installation;
pub mod plugin_storage;
pub mod project;
pub mod project_status; // Added project_status module
pub mod roadmap;
pub mod team;
pub mod workflow; // Added workflow module
pub mod workspace;
pub mod workspace_member;
pub mod workspace_user;

// Re-export all models to maintain compatibility with existing code
// This ensures that existing imports like `use crate::db::models::User` still work

// API response structures
pub use api::*;

// Authentication and user models
pub use auth::*;

// Comment models
pub use comment::*;

// Cycle models
pub use cycle::*;

// Issue models
pub use issue::*;

// Label models
pub use label::*;

// Notification models
pub use notification::*;

// Project models
pub use project::*;

// Roadmap models
pub use roadmap::*;

// Team models
pub use team::*;

// Workspace models
pub use workspace::*;

// WorkspaceMember models
pub use invitation::*;
pub use workspace_member::*;
pub use workspace_user::*;

// Workflow models
pub use workflow::*;

// Plugin system models
pub use agent_run::*;
pub use issue_field_definition::*;
pub use issue_field_value::*;
pub use plugin::*;
pub use plugin_audit::*;
pub use plugin_installation::*;
pub use plugin_storage::*;
