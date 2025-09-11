pub mod cache;
pub mod config;
pub mod db;
pub mod error;
pub mod middleware;
pub mod routes;
pub mod schema;
pub mod services;
pub mod validation;
pub mod websocket;

use crate::db::DbPool;
use crate::config::Config;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub redis: redis::Client,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(db: DbPool, redis: redis::Client, config: Config) -> Self {
        Self {
            db,
            redis,
            config: Arc::new(config),
        }
    }
}

pub fn init_tracing(config: &Config) {
    let level_filter = match config.log_level.as_str() {
        "trace" => "trace",
        "debug" => "debug",
        "info" => "info",
        "warn" => "warn",
        "error" => "error",
        _ => "info",
    };

    unsafe {
        std::env::set_var("RUST_LOG", level_filter);
    }

    match config.log_format.as_str() {
        "json" => {
            tracing_subscriber::fmt()
                .json()
                .init();
        },
        _ => {
            tracing_subscriber::fmt()
                .init();
        }
    }
}
