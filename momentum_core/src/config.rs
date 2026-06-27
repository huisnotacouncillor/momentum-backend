use crate::error::{AppError, AppResult};
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub database_url: String,
    #[serde(default = "default_max_connections")]
    pub database_max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub database_min_connections: u32,
    #[serde(default = "default_connection_timeout")]
    pub database_connection_timeout: u64,

    pub redis_url: String,
    #[serde(default = "default_redis_pool_size")]
    pub redis_pool_size: u32,

    #[serde(default = "default_host")]
    pub server_host: String,
    #[serde(default = "default_port")]
    pub server_port: u16,
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,

    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_access_token_expires")]
    pub jwt_access_token_expires_in: u64,
    #[serde(default = "default_refresh_token_expires")]
    pub jwt_refresh_token_expires_in: u64,

    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_log_format")]
    pub log_format: String,

    #[serde(default = "default_assets_url")]
    pub assets_url: String,

    #[serde(default = "default_bcrypt_cost")]
    pub bcrypt_cost: u32,
}

// 为了向后兼容，创建嵌套结构的访问器
#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
}

#[derive(Clone, Debug)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_expires_in: u64,
    pub refresh_token_expires_in: u64,
}

#[derive(Clone, Debug)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Clone, Debug)]
pub struct AssetsConfig {
    pub base_url: String,
}

// Default value functions
fn default_max_connections() -> u32 {
    20
}
fn default_min_connections() -> u32 {
    5
}
fn default_connection_timeout() -> u64 {
    30
}
fn default_redis_pool_size() -> u32 {
    20
}
fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    8000
}
fn default_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}
fn default_jwt_secret() -> String {
    "your-secret-key".to_string()
}
fn default_access_token_expires() -> u64 {
    3600
} // 1 hour
fn default_refresh_token_expires() -> u64 {
    604800
} // 7 days
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "json".to_string()
}
fn default_assets_url() -> String {
    "http://localhost:8000/assets".to_string()
}
fn default_bcrypt_cost() -> u32 {
    4
} // Further reduce cost for better performance, use 12+ for production

impl Config {
    pub fn from_env() -> AppResult<Self> {
        dotenvy::dotenv().ok();

        let config = envy::from_env::<Config>()
            .map_err(|e| AppError::Config(format!("Failed to load config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> AppResult<()> {
        if self.database_max_connections == 0 {
            return Err(AppError::Config(
                "DATABASE_MAX_CONNECTIONS must be > 0".to_string(),
            ));
        }

        if self.database_min_connections > self.database_max_connections {
            return Err(AppError::Config(
                "DATABASE_MIN_CONNECTIONS cannot be greater than DATABASE_MAX_CONNECTIONS"
                    .to_string(),
            ));
        }

        if self.redis_pool_size == 0 {
            return Err(AppError::Config("REDIS_POOL_SIZE must be > 0".to_string()));
        }

        if self.jwt_secret == "your-secret-key" {
            return Err(AppError::Config(
                "JWT_SECRET must be set to a secure value".to_string(),
            ));
        }

        if self.jwt_access_token_expires_in == 0 {
            return Err(AppError::Config(
                "JWT_ACCESS_TOKEN_EXPIRES_IN must be > 0".to_string(),
            ));
        }

        Ok(())
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }

    // 提供嵌套结构的访问器
    pub fn database(&self) -> DatabaseConfig {
        DatabaseConfig {
            url: self.database_url.clone(),
            max_connections: self.database_max_connections,
            min_connections: self.database_min_connections,
            connection_timeout: self.database_connection_timeout,
        }
    }

    pub fn redis(&self) -> RedisConfig {
        RedisConfig {
            url: self.redis_url.clone(),
            pool_size: self.redis_pool_size,
        }
    }

    pub fn server(&self) -> ServerConfig {
        ServerConfig {
            host: self.server_host.clone(),
            port: self.server_port,
            cors_origins: self.cors_origins.clone(),
        }
    }

    pub fn auth(&self) -> AuthConfig {
        AuthConfig {
            jwt_secret: self.jwt_secret.clone(),
            access_token_expires_in: self.jwt_access_token_expires_in,
            refresh_token_expires_in: self.jwt_refresh_token_expires_in,
        }
    }

    pub fn logging(&self) -> LoggingConfig {
        LoggingConfig {
            level: self.log_level.clone(),
            format: self.log_format.clone(),
        }
    }

    pub fn assets(&self) -> AssetsConfig {
        AssetsConfig {
            base_url: self.assets_url.clone(),
        }
    }
}

// 为了向后兼容，保留旧的字段访问方式
impl Config {
    pub fn db_url(&self) -> &str {
        &self.database_url
    }

    pub fn redis_url(&self) -> &str {
        &self.redis_url
    }
}
