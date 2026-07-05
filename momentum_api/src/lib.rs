// Momentum API - HTTP/WebSocket layer
pub mod config;
pub mod routes;
pub mod middleware;
pub mod websocket;
pub mod cache;
pub mod state;
pub mod validation;
pub mod error;
pub mod observability;

// Re-exports for convenience
pub use config::AppConfig;
pub use state::AppState;
pub use momentum_core::db;
pub use momentum_core::error::AppError;