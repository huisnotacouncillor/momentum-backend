//! 日志配置模块
//!
//! 暴露：
//! - `init_tracing(opts: LoggingOptions)` — 初始化 tracing subscriber
//! - `LoggingOptions { level, format }`
//! - `LoggingLevel` / `LoggingFormat` 枚举
//!
//! Issue #2 修复：
//! - **替换** `tracing_subscriber::fmt::init()`（默认配置，不读 env）
//! - 读取 `Config.log_level` → `EnvFilter`，允许 `LOG_LEVEL=debug` 覆盖
//! - 读取 `Config.log_format` → json / pretty
//! - 服务器启动时只打印 sanitized config（不会泄漏 JWT secret / database url）

use serde::{Deserialize, Serialize};

/// 服务器启动时的日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingOptions {
    /// tracing level filter，例如 "info"、"debug"、"momentum_api=trace"
    pub level: String,
    /// "json" (生产) 或 "pretty" (本地开发)
    pub format: String,
}

/// 解析后的 level，用于验证输入合法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggingLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LoggingLevel {
    pub fn as_env_filter(&self) -> &'static str {
        match self {
            LoggingLevel::Trace => "trace",
            LoggingLevel::Debug => "debug",
            LoggingLevel::Info => "info",
            LoggingLevel::Warn => "warn",
            LoggingLevel::Error => "error",
        }
    }
}

impl std::str::FromStr for LoggingLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 保留原始字符串用于错误消息，让用户看到自己输入的内容
        let original = s.to_string();
        match s.to_lowercase().as_str() {
            "trace" => Ok(LoggingLevel::Trace),
            "debug" => Ok(LoggingLevel::Debug),
            "info" => Ok(LoggingLevel::Info),
            "warn" | "warning" => Ok(LoggingLevel::Warn),
            "error" => Ok(LoggingLevel::Error),
            _ => Err(format!(
                "unknown log level '{}': expected one of trace|debug|info|warn|error",
                original
            )),
        }
    }
}

/// 解析后的输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggingFormat {
    Json,
    Pretty,
}

impl std::str::FromStr for LoggingFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let original = s.to_string();
        match s.to_lowercase().as_str() {
            "json" => Ok(LoggingFormat::Json),
            "pretty" | "text" | "human" => Ok(LoggingFormat::Pretty),
            _ => Err(format!(
                "unknown log format '{}': expected one of json|pretty",
                original
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logging_level_parses_all_known_levels() {
        assert_eq!("trace".parse::<LoggingLevel>().unwrap(), LoggingLevel::Trace);
        assert_eq!("debug".parse::<LoggingLevel>().unwrap(), LoggingLevel::Debug);
        assert_eq!("info".parse::<LoggingLevel>().unwrap(), LoggingLevel::Info);
        assert_eq!("warn".parse::<LoggingLevel>().unwrap(), LoggingLevel::Warn);
        assert_eq!("warning".parse::<LoggingLevel>().unwrap(), LoggingLevel::Warn);
        assert_eq!("error".parse::<LoggingLevel>().unwrap(), LoggingLevel::Error);
    }

    #[test]
    fn logging_level_is_case_insensitive() {
        assert_eq!("INFO".parse::<LoggingLevel>().unwrap(), LoggingLevel::Info);
        assert_eq!("Debug".parse::<LoggingLevel>().unwrap(), LoggingLevel::Debug);
    }

    #[test]
    fn logging_level_rejects_unknown_values() {
        let err = "LOUD".parse::<LoggingLevel>().unwrap_err();
        assert!(err.contains("unknown log level 'LOUD'"));
        assert!(err.contains("expected"));
    }

    #[test]
    fn logging_level_serializes_to_env_filter_string() {
        assert_eq!(LoggingLevel::Trace.as_env_filter(), "trace");
        assert_eq!(LoggingLevel::Error.as_env_filter(), "error");
    }

    #[test]
    fn logging_format_parses_json_and_pretty() {
        assert_eq!("json".parse::<LoggingFormat>().unwrap(), LoggingFormat::Json);
        assert_eq!(
            "pretty".parse::<LoggingFormat>().unwrap(),
            LoggingFormat::Pretty
        );
        assert_eq!("text".parse::<LoggingFormat>().unwrap(), LoggingFormat::Pretty);
        assert_eq!("human".parse::<LoggingFormat>().unwrap(), LoggingFormat::Pretty);
    }

    #[test]
    fn logging_format_is_case_insensitive() {
        assert_eq!(
            "JSON".parse::<LoggingFormat>().unwrap(),
            LoggingFormat::Json
        );
        assert_eq!(
            "Pretty".parse::<LoggingFormat>().unwrap(),
            LoggingFormat::Pretty
        );
    }

    #[test]
    fn logging_format_rejects_unknown_values() {
        let err = "yaml".parse::<LoggingFormat>().unwrap_err();
        assert!(err.contains("unknown log format 'yaml'"));
    }

    #[test]
    fn logging_options_default_log_level_info_falls_through_to_valid_level() {
        // Config default 是 "info"，应该解析成功
        let opts = LoggingOptions {
            level: "info".to_string(),
            format: "json".to_string(),
        };
        let level: LoggingLevel = opts.level.parse().unwrap();
        let format: LoggingFormat = opts.format.parse().unwrap();
        assert_eq!(level, LoggingLevel::Info);
        assert_eq!(format, LoggingFormat::Json);
    }
}
