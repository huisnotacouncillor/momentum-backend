//! API Configuration
//!
//! This module provides configuration for the API server.

use momentum_core::config::Config;
use serde::Serialize;
use std::net::SocketAddr;

/// API Configuration that wraps core config
#[derive(Debug, Clone, Serialize)]
pub struct AppConfig {
    pub server_host: String,
    pub server_port: u16,
    pub server_address: SocketAddr,
    pub cors_origins: Vec<String>,
    pub db_url: String,
    pub redis_url: String,
    pub assets_url: String,
}

impl AppConfig {
    pub fn from_core_config(core: Config) -> Self {
        let addr: SocketAddr = format!("{}:{}", core.server_host, core.server_port)
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:8000".parse().unwrap());

        Self {
            server_host: core.server_host,
            server_port: core.server_port,
            server_address: addr,
            cors_origins: core.cors_origins,
            db_url: core.database_url,
            redis_url: core.redis_url,
            assets_url: core.assets_url,
        }
    }

    pub fn assets(&self) -> momentum_core::config::AssetsConfig {
        momentum_core::config::AssetsConfig {
            base_url: self.assets_url.clone(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".to_string(),
            server_port: 8000,
            server_address: "127.0.0.1:8000".parse().unwrap(),
            cors_origins: vec!["*".to_string()],
            db_url: "postgres://localhost/momentum".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            assets_url: "http://localhost:8000/assets".to_string(),
        }
    }
}
