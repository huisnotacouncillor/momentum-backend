// Sub-modules organized by functional domain
pub mod api;
pub mod auth;
pub mod comment;
pub mod cycle;
pub mod invitation;
pub mod issue;
pub mod label;
pub mod project;
pub mod project_status; // Added project_status module
pub mod roadmap;
pub mod team;
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

// Project models
pub use project::*;

// Roadmap models
pub use roadmap::*;

// Team models
pub use team::*;

// Workspace models
pub use workspace::*;

// WorkspaceMember models
pub use workspace_member::*;
pub use workspace_user::*;
pub use invitation::*;