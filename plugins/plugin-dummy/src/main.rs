//! Dummy Plugin
//!
//! Momentum 内部第一个插件，验证 SDK 全流程。
//! 启动后监听 Unix Domain Socket，接收 Core 的 gRPC 调用。
//!
//! 运行：
//!   MOMENTUM_SOCKET_PATH=/tmp/momentum-plugins/dummy-plugin-test.sock \
//!   MOMENTUM_PLUGIN_ID=dummy-plugin \
//!   MOMENTUM_WORKSPACE_ID=00000000-0000-0000-0000-000000000000 \
//!   cargo run --bin plugin-dummy

use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};

pub mod proto {
    tonic::include_proto!("momentum.plugin.v1");
}

use proto::{
    plugin_service_server::{PluginService, PluginServiceServer},
    Empty, HandshakeRequest, HandshakeResponse, HeartbeatRequest, HeartbeatResponse,
    InvokeAgentRequest, InvokeAgentResponse, OnFieldWriteRequest, OnFieldWriteResponse,
    PublishEventRequest, StorageGetRequest, StorageGetResponse, StoragePutRequest,
    StoragePutResponse,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct DummyPluginState {
    /// 简单内存 KV
    storage: RwLock<HashMap<String, serde_json::Value>>,
}

#[derive(Default)]
pub struct DummyPlugin {
    state: Arc<DummyPluginState>,
}

#[allow(dead_code)]
type ResponseStream<T> = Pin<Box<dyn futures::Stream<Item = Result<T, Status>> + Send>>;

#[tonic::async_trait]
impl PluginService for DummyPlugin {
    async fn handshake(
        &self,
        request: Request<HandshakeRequest>,
    ) -> Result<Response<HandshakeResponse>, Status> {
        let req = request.into_inner();
        tracing::info!(
            "Handshake: plugin_id={}, version={}, workspace={}",
            req.plugin_id,
            req.plugin_version,
            req.workspace_id
        );
        Ok(Response::new(HandshakeResponse {
            ok: true,
            error: String::new(),
            extensions: vec![],
        }))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        Ok(Response::new(HeartbeatResponse {
            ts_ms: request.into_inner().ts_ms,
            server_ts_ms: chrono::Utc::now().timestamp_millis(),
        }))
    }

    async fn on_field_write(
        &self,
        request: Request<OnFieldWriteRequest>,
    ) -> Result<Response<OnFieldWriteResponse>, Status> {
        let req = request.into_inner();
        tracing::info!(
            "OnFieldWrite: issue={}, field={}, value={:?}",
            req.issue_id,
            req.field_key,
            req.new_value
        );

        // dummy 校验：effort 不能超过 100
        if req.field_key == "issue.effort" {
            if let Some(n) = extract_number(req.new_value.as_ref()) {
                if n > 100.0 {
                    return Ok(Response::new(OnFieldWriteResponse {
                        ok: false,
                        error: "effort cannot exceed 100".to_string(),
                        rejected_value: req.new_value,
                    }));
                }
            }
        }
        Ok(Response::new(OnFieldWriteResponse {
            ok: true,
            error: String::new(),
            rejected_value: None,
        }))
    }

    async fn invoke_agent(
        &self,
        request: Request<InvokeAgentRequest>,
    ) -> Result<Response<InvokeAgentResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("InvokeAgent: agent={}", req.agent_id);

        // dummy-agent: 回显 input.message
        let msg = req
            .input
            .as_ref()
            .and_then(|s| s.fields.get("message"))
            .and_then(extract_string)
            .unwrap_or_else(|| "(no message)".to_string());

        let output_json = serde_json::json!({
            "reply": format!("dummy-agent received: {}", msg),
            "echo_of": msg,
        });
        let output_struct = json_to_struct(&output_json);

        Ok(Response::new(InvokeAgentResponse {
            ok: true,
            error: String::new(),
            output: output_struct,
            tokens_in: 10,
            tokens_out: 20,
            duration_ms: 50,
        }))
    }

    async fn storage_get(
        &self,
        request: Request<StorageGetRequest>,
    ) -> Result<Response<StorageGetResponse>, Status> {
        let req = request.into_inner();
        let k = format!("{}/{}/{}", req.workspace_id, req.namespace, req.key);
        let storage = self.state.storage.read().await;
        if let Some(v) = storage.get(&k) {
            Ok(Response::new(StorageGetResponse {
                found: true,
                value: json_to_value(v),
            }))
        } else {
            Ok(Response::new(StorageGetResponse {
                found: false,
                value: None,
            }))
        }
    }

    async fn storage_put(
        &self,
        request: Request<StoragePutRequest>,
    ) -> Result<Response<StoragePutResponse>, Status> {
        let req = request.into_inner();
        let k = format!("{}/{}/{}", req.workspace_id, req.namespace, req.key);
        if let Some(v) = req.value {
            let json = value_to_json(&v);
            let mut storage = self.state.storage.write().await;
            storage.insert(k, json);
        }
        Ok(Response::new(StoragePutResponse { ok: true }))
    }

    async fn publish_event(
        &self,
        request: Request<PublishEventRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        tracing::info!("publish_event: type={}", req.event_type);
        Ok(Response::new(Empty {}))
    }
}

// === JSON ↔ prost types 转换 ===

fn json_to_struct(v: &serde_json::Value) -> Option<prost_types::Struct> {
    let obj = v.as_object()?;
    let mut fields = std::collections::BTreeMap::new();
    for (k, val) in obj {
        fields.insert(k.clone(), json_to_value(val)?);
    }
    Some(prost_types::Struct { fields })
}

fn json_to_value(v: &serde_json::Value) -> Option<prost_types::Value> {
    use prost_types::value::Kind;
    use prost_types::{ListValue, Struct};
    let kind = match v {
        serde_json::Value::Null => Kind::NullValue(0),
        serde_json::Value::Bool(b) => Kind::BoolValue(*b),
        serde_json::Value::Number(n) => Kind::NumberValue(n.as_f64()?),
        serde_json::Value::String(s) => Kind::StringValue(s.clone()),
        serde_json::Value::Array(arr) => {
            let values: Option<Vec<prost_types::Value>> = arr.iter().map(json_to_value).collect();
            Kind::ListValue(ListValue { values: values? })
        }
        serde_json::Value::Object(obj) => {
            let mut fields = std::collections::BTreeMap::new();
            for (k, val) in obj {
                fields.insert(k.clone(), json_to_value(val)?);
            }
            Kind::StructValue(Struct { fields })
        }
    };
    Some(prost_types::Value { kind: Some(kind) })
}

fn value_to_json(v: &prost_types::Value) -> serde_json::Value {
    use prost_types::value::Kind;
    match &v.kind {
        Some(Kind::NullValue(_)) | None => serde_json::Value::Null,
        Some(Kind::BoolValue(b)) => serde_json::Value::Bool(*b),
        Some(Kind::NumberValue(n)) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Some(Kind::StringValue(s)) => serde_json::Value::String(s.clone()),
        Some(Kind::ListValue(lv)) => {
            serde_json::Value::Array(lv.values.iter().map(value_to_json).collect())
        }
        Some(Kind::StructValue(sv)) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in &sv.fields {
                obj.insert(k.clone(), value_to_json(val));
            }
            serde_json::Value::Object(obj)
        }
    }
}

fn extract_number(v: Option<&prost_types::Value>) -> Option<f64> {
    use prost_types::value::Kind;
    match v?.kind {
        Some(Kind::NumberValue(n)) => Some(n),
        _ => None,
    }
}

fn extract_string(v: &prost_types::Value) -> Option<String> {
    use prost_types::value::Kind;
    match &v.kind {
        Some(Kind::StringValue(s)) => Some(s.clone()),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let socket_path = std::env::var("MOMENTUM_SOCKET_PATH")
        .unwrap_or_else(|_| "/tmp/momentum-plugins/dummy-plugin.sock".to_string());
    let plugin_id =
        std::env::var("MOMENTUM_PLUGIN_ID").unwrap_or_else(|_| "dummy-plugin".to_string());
    let workspace_id = std::env::var("MOMENTUM_WORKSPACE_ID")
        .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000000".to_string());

    // 简化：v0.1 用 TCP localhost 端口（设计文档是 unix socket，v0.2 改）
    // 端口约定：9991 = dummy-plugin
    let port: u16 = std::env::var("MOMENTUM_PLUGIN_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9991);
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    tracing::info!(
        "dummy plugin starting: id={}, workspace={}, port={} (legacy socket: {})",
        plugin_id,
        workspace_id,
        port,
        socket_path
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let plugin = DummyPlugin::default();
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
    Server::builder()
        .add_service(PluginServiceServer::new(plugin))
        .serve_with_incoming(incoming)
        .await?;

    Ok(())
}
