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
    /// 仅供测试使用的、必定返回有效 Config 的构造器。
    /// 关键字段填上合法占位值，避免 `Config::default` 触发 `validate()` 失败。
    #[cfg(test)]
    pub fn default_for_test() -> Self {
        let mut c = Self {
            database_url: "postgres://localhost/test".to_string(),
            database_max_connections: 5,
            database_min_connections: 1,
            database_connection_timeout: 30,
            redis_url: "redis://localhost".to_string(),
            redis_pool_size: 5,
            server_host: "127.0.0.1".to_string(),
            server_port: 8000,
            cors_origins: vec!["*".to_string()],
            jwt_secret: "test-secret-32-chars-or-more-here".to_string(),
            jwt_access_token_expires_in: 3600,
            jwt_refresh_token_expires_in: 604800,
            log_level: "info".to_string(),
            log_format: "json".to_string(),
            assets_url: "http://localhost:8000/assets".to_string(),
            bcrypt_cost: 12,
        };
        c.validate().expect("default_for_test must produce a valid Config");
        c
    }

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

/// 用于日志/调试输出的精简版配置 —— 显式不包含 jwt_secret / database_url。
#[derive(Debug, Clone, serde::Serialize)]
pub struct SanitizedConfig {
    pub server_host: String,
    pub server_port: u16,
    pub log_level: String,
    pub log_format: String,
    pub cors_origins: Vec<String>,
    pub bcrypt_cost: u32,
    pub redis_pool_size: u32,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub database_connection_timeout: u64,
    pub access_token_expires: u64,
    pub refresh_token_expires: u64,
    pub assets_url: String,
    /// 仅做"是否存在"披露（运维排错用），不输出值
    #[serde(rename = "auth_key_present")]
    pub auth_key_present: bool,
}

impl Config {
    /// 构造不含敏感信息的配置快照，供 `tracing::info!`、审计、调试输出使用。
    pub fn sanitize_for_logging(&self) -> SanitizedConfig {
        SanitizedConfig {
            server_host: self.server_host.clone(),
            server_port: self.server_port,
            log_level: self.log_level.clone(),
            log_format: self.log_format.clone(),
            cors_origins: self.cors_origins.clone(),
            bcrypt_cost: self.bcrypt_cost,
            redis_pool_size: self.redis_pool_size,
            database_max_connections: self.database_max_connections,
            database_min_connections: self.database_min_connections,
            database_connection_timeout: self.database_connection_timeout,
            access_token_expires: self.jwt_access_token_expires_in,
            refresh_token_expires: self.jwt_refresh_token_expires_in,
            assets_url: self.assets_url.clone(),
            auth_key_present: !self.jwt_secret.is_empty()
                && self.jwt_secret != "your-secret-key",
        }
    }
}

#[cfg(test)]
mod sanitization_tests {
    use super::*;
    use serde_json::json;

    fn make_config_with_secrets() -> Config {
        // 直接构造，避开 envy::from_str 不会带 env var 的问题
        // 借用 dotenv 来加载 .env 也不可靠 — 测试应纯函数化
        // 改用对 Config::default() 的字段进行覆盖（最小化测试依赖）
        let mut c = Config::default_for_test();
        c.jwt_secret = "actual-super-secret-jwt-string".to_string();
        c.database_url = "postgres://admin:hunter2@db.internal.example.com:5432/rust_backend"
            .to_string();
        c.bcrypt_cost = 12;
        c
    }

    #[test]
    fn sanitize_for_logging_excludes_jwt_secret() {
        let config = make_config_with_secrets();
        let sanitized = config.sanitize_for_logging();
        let json = serde_json::to_string(&sanitized).unwrap();
        assert!(
            !json.contains("actual-super-secret-jwt-string"),
            "sanitized output must not include jwt_secret value. got: {}",
            json
        );
        assert!(
            !json.to_lowercase().contains("jwt_secret"),
            "sanitized output must not even mention jwt_secret key. got: {}",
            json
        );
    }

    #[test]
    fn sanitize_for_logging_excludes_database_url() {
        let config = make_config_with_secrets();
        let sanitized = config.sanitize_for_logging();
        let json = serde_json::to_string(&sanitized).unwrap();
        assert!(
            !json.contains("hunter2"),
            "sanitized output must not include database password. got: {}",
            json
        );
        assert!(
            !json.contains("admin@db.internal.example.com"),
            "sanitized output must not include database host with creds. got: {}",
            json
        );
    }

    #[test]
    fn sanitize_for_logging_reports_auth_key_present_status() {
        let config = make_config_with_secrets();
        let sanitized = config.sanitize_for_logging();
        let json = serde_json::to_value(&sanitized).unwrap();
        let status = json
            .get("auth_key_present")
            .expect("auth_key_present must be present");
        assert_eq!(status, &json!(true));
    }

    #[test]
    fn sanitize_for_logging_includes_safe_fields() {
        let config = make_config_with_secrets();
        let sanitized = config.sanitize_for_logging();
        let json: serde_json::Value = serde_json::to_value(&sanitized).unwrap();
        for field in &[
            "log_level",
            "log_format",
            "bcrypt_cost",
            "cors_origins",
            "server_host",
            "server_port",
        ] {
            assert!(
                json.get(field).is_some(),
                "sanitized output must include {} for ops visibility. got: {}",
                field,
                json
            );
        }
    }

    #[test]
    fn sanitize_for_logging_handles_empty_cors_origins() {
        let mut config = make_config_with_secrets();
        config.cors_origins = vec![];
        let sanitized = config.sanitize_for_logging();
        let json = serde_json::to_string(&sanitized).unwrap();
        assert!(json.contains("cors_origins"));
    }

    #[test]
    fn sanitize_for_logging_no_secrets_in_debug() {
        let config = make_config_with_secrets();
        let sanitized = config.sanitize_for_logging();
        let dbg = format!("{:?}", sanitized);
        for forbidden in &["actual-super-secret-jwt-string", "hunter2"] {
            assert!(
                !dbg.contains(forbidden),
                "leaked in Debug output: {} in {}",
                forbidden,
                dbg
            );
        }
    }

    #[test]
    fn sanitize_for_logging_detects_unset_auth_key() {
        let mut config = make_config_with_secrets();
        config.jwt_secret = "your-secret-key".to_string();
        let sanitized = config.sanitize_for_logging();
        assert!(!sanitized.auth_key_present);
    }
}
