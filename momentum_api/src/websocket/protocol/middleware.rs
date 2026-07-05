//! ProtocolVersion 中间件（spec §7.2）
//!
//! 从 `envelope.metadata["ws_version"]` 读取客户端版本，与服务器支持的版本列表进行协商：
//! - 客户端版本不可解析 -> 默认 v1.0（向后兼容）
//! - 客户端版本不在 supported() 列表里 -> 拒绝
//! - 主版本不同 -> 拒绝
//!
//! Step 9：之前临时占用 `RequestContext.idempotency_key` 的 hack 已废弃；
//! metadata 现在正经存在于 `CommandEnvelope`，后续真正把协议版本塞进
//! ws client → server 的连接 header / 首帧即可。

use async_trait::async_trait;
use serde_json::Value;

use momentum_core::error::AppError;

use crate::websocket::middleware::{
    CommandEnvelope, CommandMiddleware, MiddlewareContext, NextMiddleware,
};
use super::version::{ProtocolVersion, VersionNegotiation};

pub struct VersionNegotiationMiddleware {
    supported: Vec<ProtocolVersion>,
    default: ProtocolVersion,
}

impl VersionNegotiationMiddleware {
    pub fn new() -> Self {
        Self {
            supported: ProtocolVersion::supported().to_vec(),
            default: ProtocolVersion::latest(),
        }
    }

    fn parse_client_version(&self, envelope: &CommandEnvelope) -> ProtocolVersion {
        envelope
            .metadata
            .get("ws_version")
            .and_then(|v| ProtocolVersion::parse(v))
            .unwrap_or(ProtocolVersion::V1_0)
    }
}

impl Default for VersionNegotiationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandMiddleware for VersionNegotiationMiddleware {
    fn name(&self) -> &'static str {
        "version_negotiation"
    }

    async fn process(
        &self,
        envelope: CommandEnvelope,
        _ctx: &MiddlewareContext,
        next: NextMiddleware<'_>,
    ) -> Result<Value, AppError> {
        let client = self.parse_client_version(&envelope);
        // 不在 supported() 列表中
        if !self.supported.contains(&client) {
            return Err(AppError::Internal(format!(
                "UNSUPPORTED_VERSION: client sent {:?}, server supports {:?}",
                client, self.supported
            )));
        }
        // 与默认协商
        let n = VersionNegotiation::negotiate(client, self.default);
        if !n.is_compatible {
            return Err(AppError::Internal(format!(
                "VERSION_MISMATCH: client={:?} server={:?}",
                client, self.default
            )));
        }
        next.run().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::feature_flags::FeatureFlags;
    use crate::websocket::middleware::MiddlewareChain;
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;
    use momentum_core::services::context::RequestContext;

    fn make_env(version: Option<&str>) -> CommandEnvelope {
        let mut env = CommandEnvelope::new(
            "ping",
            json!({}),
            RequestContext {
                user_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                idempotency_key: None,
        trace_id: "unknown".to_string(),
            },
            Some("req".into()),
        );
        if let Some(v) = version {
            env.metadata.insert("ws_version".to_string(), v.to_string());
        }
        env
    }

    fn make_ctx() -> MiddlewareContext {
        MiddlewareContext {
            feature_flags: Arc::new(FeatureFlags::default()),
        }
    }

    #[tokio::test]
    async fn default_version_passes() {
        let env = make_env(None); // -> v1.0 default
        let chain = MiddlewareChain::new().push(VersionNegotiationMiddleware::new());
        let out = chain.execute(env, &make_ctx()).await.unwrap();
        assert_eq!(out, json!({}));
    }

    #[tokio::test]
    async fn supported_v1_1_passes() {
        let env = make_env(Some("1.1"));
        let chain = MiddlewareChain::new().push(VersionNegotiationMiddleware::new());
        let out = chain.execute(env, &make_ctx()).await.unwrap();
        assert_eq!(out, json!({}));
    }

    #[tokio::test]
    async fn unsupported_version_blocked() {
        let env = make_env(Some("9.9"));
        let chain = MiddlewareChain::new().push(VersionNegotiationMiddleware::new());
        let err = chain.execute(env, &make_ctx()).await.unwrap_err();
        match err {
            AppError::Internal(m) => assert!(m.contains("UNSUPPORTED_VERSION")),
            other => panic!("expected Internal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn malformed_string_falls_back_to_default() {
        let env = make_env(Some("not-a-version"));
        let chain = MiddlewareChain::new().push(VersionNegotiationMiddleware::new());
        // 解析失败 -> 视为 v1.0 -> 通过
        let out = chain.execute(env, &make_ctx()).await.unwrap();
        assert_eq!(out, json!({}));
    }
}
