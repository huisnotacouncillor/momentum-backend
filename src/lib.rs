pub mod cache;
pub mod config;
pub mod db;
pub mod error;
pub mod middleware;
pub mod routes;
pub mod schema;
pub mod services;
pub mod utils;
pub mod validation;
pub mod websocket;

use crate::db::DbPool;
use crate::config::Config;
use crate::utils::AssetUrlHelper;
use crate::middleware::auth::{AuthService, AuthConfig};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub redis: redis::Client,
    pub config: Arc<Config>,
    pub asset_helper: AssetUrlHelper,
    pub auth_service: AuthService,
}

impl AppState {
    pub fn new(db: DbPool, redis: redis::Client, config: Config) -> Self {
        let asset_helper = AssetUrlHelper::new(&config.assets());
        let auth_service = AuthService::new(AuthConfig::default());
        Self {
            db,
            redis,
            config: Arc::new(config),
            asset_helper,
            auth_service,
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
