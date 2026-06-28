//! Plugin 错误类型

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("manifest invalid: {0}")]
    ManifestInvalid(String),

    #[error("plugin not found: {0}")]
    NotFound(String),

    #[error("plugin already installed: {0}")]
    AlreadyInstalled(String),

    #[error("plugin not enabled: {0}")]
    NotEnabled(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("process spawn failed: {0}")]
    ProcessSpawn(String),

    #[error("handshake failed: {0}")]
    Handshake(String),

    #[error("gRPC call failed: {0}")]
    GrpcCall(String),

    #[error("db error: {0}")]
    Db(String),

    #[error("field not registered: {0}")]
    FieldNotRegistered(String),

    #[error("agent not registered: {0}")]
    AgentNotRegistered(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<diesel::result::Error> for PluginError {
    fn from(e: diesel::result::Error) -> Self {
        PluginError::Db(e.to_string())
    }
}

impl From<serde_yaml::Error> for PluginError {
    fn from(e: serde_yaml::Error) -> Self {
        PluginError::ManifestInvalid(e.to_string())
    }
}

pub type PluginResult<T> = Result<T, PluginError>;
