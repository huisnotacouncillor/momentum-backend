use crate::{
    db::DbPool, db::enums::LabelLevel, error::AppError, services::context::RequestContext,
    validation::label::validate_create_label, websocket::security::SecureMessage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
// 直接使用 tracing:: 前缀，避免导入冲突

/// WebSocket命令类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSocketCommand {
    /// 创建标签命令
    CreateLabel {
        data: CreateLabelCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// 更新标签命令
    UpdateLabel {
        label_id: Uuid,
        data: UpdateLabelCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// 删除标签命令
    DeleteLabel {
        label_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// 查询标签命令
    QueryLabels {
        filters: LabelFilters,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// 批量创建标签命令
    BatchCreateLabels {
        data: Vec<CreateLabelCommand>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// 批量更新标签命令
    BatchUpdateLabels {
        updates: Vec<LabelUpdate>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// 批量删除标签命令
    BatchDeleteLabels {
        label_ids: Vec<Uuid>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// 订阅主题命令
    Subscribe {
        topics: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// 取消订阅主题命令
    Unsubscribe {
        topics: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// 获取连接信息命令
    GetConnectionInfo {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// Ping命令
    Ping {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
}

/// 创建标签命令数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLabelCommand {
    pub name: String,
    pub color: String,
    pub level: LabelLevel,
}

/// 更新标签命令数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLabelCommand {
    pub name: Option<String>,
    pub color: Option<String>,
    pub level: Option<LabelLevel>,
}

/// 标签查询过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelFilters {
    pub workspace_id: Option<Uuid>,
    pub level: Option<LabelLevel>,
    pub name_pattern: Option<String>,
    pub color: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// 标签更新信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelUpdate {
    pub label_id: Uuid,
    pub data: UpdateLabelCommand,
}

/// 连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub user_id: Uuid,
    pub username: String,
    pub connected_at: DateTime<Utc>,
    pub last_ping: DateTime<Utc>,
    pub subscriptions: Vec<String>,
    pub message_queue_size: usize,
    pub state: String,
}

/// WebSocket命令响应 - 统一响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketCommandResponse {
    /// 命令类型标识
    pub command_type: String,
    /// 幂等性键（后端生成）
    pub idempotency_key: String,
    /// 请求ID（前端传入，用于跟踪）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// 执行状态
    pub success: bool,
    /// 响应数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WebSocketCommandError>,
    /// 响应元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<WebSocketResponseMeta>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// WebSocket响应元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketResponseMeta {
    /// 执行时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
    /// 分页信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<WebSocketPagination>,
    /// 总数统计
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<i64>,
    /// 批量操作统计
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_stats: Option<WebSocketBatchStats>,
    /// 业务特定元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_meta: Option<serde_json::Value>,
}

/// WebSocket分页信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketPagination {
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

/// WebSocket批量操作统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketBatchStats {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
    pub skipped: i64,
}

/// WebSocket命令错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketCommandError {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 错误字段（用于验证错误）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// 错误详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// 错误类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
}

// 便捷构造函数
impl WebSocketCommandResponse {
    /// 成功响应
    pub fn success(
        command_type: &str,
        idempotency_key: &str,
        request_id: Option<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            command_type: command_type.to_string(),
            idempotency_key: idempotency_key.to_string(),
            request_id,
            success: true,
            data: Some(data),
            error: None,
            meta: None,
            timestamp: Utc::now(),
        }
    }

    /// 成功响应（带元数据）
    pub fn success_with_meta(
        command_type: &str,
        idempotency_key: &str,
        request_id: Option<String>,
        data: serde_json::Value,
        meta: WebSocketResponseMeta,
    ) -> Self {
        Self {
            command_type: command_type.to_string(),
            idempotency_key: idempotency_key.to_string(),
            request_id,
            success: true,
            data: Some(data),
            error: None,
            meta: Some(meta),
            timestamp: Utc::now(),
        }
    }

    /// 错误响应
    pub fn error(
        command_type: &str,
        idempotency_key: &str,
        request_id: Option<String>,
        error: WebSocketCommandError,
    ) -> Self {
        Self {
            command_type: command_type.to_string(),
            idempotency_key: idempotency_key.to_string(),
            request_id,
            success: false,
            data: None,
            error: Some(error),
            meta: None,
            timestamp: Utc::now(),
        }
    }

    /// 简单成功响应（无数据）
    pub fn ok(
        command_type: &str,
        idempotency_key: &str,
        request_id: Option<String>,
        message: &str,
    ) -> Self {
        Self {
            command_type: command_type.to_string(),
            idempotency_key: idempotency_key.to_string(),
            request_id,
            success: true,
            data: Some(serde_json::json!({"message": message})),
            error: None,
            meta: None,
            timestamp: Utc::now(),
        }
    }
}

impl WebSocketCommandError {
    /// 创建验证错误
    pub fn validation_error(field: &str, message: &str) -> Self {
        Self {
            code: "VALIDATION_ERROR".to_string(),
            message: message.to_string(),
            field: Some(field.to_string()),
            details: None,
            error_type: Some("validation".to_string()),
        }
    }

    /// 创建业务错误
    pub fn business_error(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field: None,
            details: None,
            error_type: Some("business".to_string()),
        }
    }

    /// 创建系统错误
    pub fn system_error(message: &str) -> Self {
        Self {
            code: "SYSTEM_ERROR".to_string(),
            message: message.to_string(),
            field: None,
            details: None,
            error_type: Some("system".to_string()),
        }
    }

    /// 创建权限错误
    pub fn permission_error(message: &str) -> Self {
        Self {
            code: "PERMISSION_ERROR".to_string(),
            message: message.to_string(),
            field: None,
            details: None,
            error_type: Some("permission".to_string()),
        }
    }

    /// 创建未找到错误
    pub fn not_found(resource: &str) -> Self {
        Self {
            code: "NOT_FOUND".to_string(),
            message: format!("{} not found", resource),
            field: None,
            details: None,
            error_type: Some("not_found".to_string()),
        }
    }
}

impl WebSocketResponseMeta {
    /// 创建带执行时间的元数据
    pub fn with_execution_time(execution_time_ms: u64) -> Self {
        Self {
            execution_time_ms: Some(execution_time_ms),
            pagination: None,
            total_count: None,
            batch_stats: None,
            business_meta: None,
        }
    }

    /// 创建带分页的元数据
    pub fn with_pagination(pagination: WebSocketPagination) -> Self {
        Self {
            execution_time_ms: None,
            pagination: Some(pagination),
            total_count: None,
            batch_stats: None,
            business_meta: None,
        }
    }

    /// 创建带批量统计的元数据
    pub fn with_batch_stats(batch_stats: WebSocketBatchStats) -> Self {
        Self {
            execution_time_ms: None,
            pagination: None,
            total_count: None,
            batch_stats: Some(batch_stats),
            business_meta: None,
        }
    }

    /// 创建带业务元数据的元数据
    pub fn with_business_meta(business_meta: serde_json::Value) -> Self {
        Self {
            execution_time_ms: None,
            pagination: None,
            total_count: None,
            batch_stats: None,
            business_meta: Some(business_meta),
        }
    }
}

impl WebSocketBatchStats {
    /// 创建批量统计
    pub fn new(total: i64, successful: i64, failed: i64, skipped: i64) -> Self {
        Self {
            total,
            successful,
            failed,
            skipped,
        }
    }
}

/// 幂等性控制
#[derive(Debug, Clone)]
pub struct IdempotencyControl {
    /// 存储已处理的命令结果
    processed_commands: Arc<RwLock<HashMap<String, WebSocketCommandResponse>>>,
    /// 命令过期时间（秒）
    expiration_seconds: u64,
}

impl IdempotencyControl {
    pub fn new(expiration_seconds: u64) -> Self {
        Self {
            processed_commands: Arc::new(RwLock::new(HashMap::new())),
            expiration_seconds,
        }
    }

    /// 检查命令是否已处理
    pub async fn is_processed(&self, idempotency_key: &str) -> Option<WebSocketCommandResponse> {
        let commands = self.processed_commands.read().await;
        commands.get(idempotency_key).cloned()
    }

    /// 标记命令为已处理
    pub async fn mark_processed(
        &self,
        idempotency_key: String,
        response: WebSocketCommandResponse,
    ) {
        let mut commands = self.processed_commands.write().await;
        commands.insert(idempotency_key, response);
    }

    /// 清理过期的命令记录
    pub async fn cleanup_expired(&self) {
        let cutoff_time = Utc::now() - chrono::Duration::seconds(self.expiration_seconds as i64);
        let mut commands = self.processed_commands.write().await;
        commands.retain(|_, response| response.timestamp > cutoff_time);
    }
}

/// WebSocket命令处理器
#[derive(Clone)]
pub struct WebSocketCommandHandler {
    db: Arc<DbPool>,
    idempotency: IdempotencyControl,
    message_signer: Option<Arc<crate::websocket::MessageSigner>>,
}

impl WebSocketCommandHandler {
    pub fn new(db: Arc<DbPool>) -> Self {
        Self {
            db,
            idempotency: IdempotencyControl::new(300), // 5分钟过期
            message_signer: None,
        }
    }

    pub fn with_message_signer(mut self, signer: Arc<crate::websocket::MessageSigner>) -> Self {
        self.message_signer = Some(signer);
        self
    }

    /// 验证安全消息
    async fn verify_secure_message(&self, secure_message: &SecureMessage) -> Result<(), AppError> {
        if let Some(ref signer) = self.message_signer {
            signer
                .verify_message(secure_message)
                .await
                .map_err(|e| AppError::auth(format!("Security verification failed: {}", e)))
        } else {
            Err(AppError::Internal(
                "Message signer not configured".to_string(),
            ))
        }
    }

    /// 生成幂等性key
    fn generate_idempotency_key(
        &self,
        command: &WebSocketCommand,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // 基于用户ID和工作区ID
        user.user_id.hash(&mut hasher);
        if let Some(workspace_id) = user.current_workspace_id {
            workspace_id.hash(&mut hasher);
        }

        // 基于命令内容生成hash
        match command {
            WebSocketCommand::CreateLabel { data, .. } => {
                "create_label".hash(&mut hasher);
                data.name.hash(&mut hasher);
                data.color.hash(&mut hasher);
                format!("{:?}", data.level).hash(&mut hasher);
            }
            WebSocketCommand::UpdateLabel { label_id, data, .. } => {
                "update_label".hash(&mut hasher);
                label_id.hash(&mut hasher);
                if let Some(ref name) = data.name {
                    name.hash(&mut hasher);
                }
                if let Some(ref color) = data.color {
                    color.hash(&mut hasher);
                }
                if let Some(ref level) = data.level {
                    format!("{:?}", level).hash(&mut hasher);
                }
            }
            WebSocketCommand::DeleteLabel { label_id, .. } => {
                "delete_label".hash(&mut hasher);
                label_id.hash(&mut hasher);
            }
            WebSocketCommand::QueryLabels { filters, .. } => {
                "query_labels".hash(&mut hasher);
                if let Some(ref level) = filters.level {
                    format!("{:?}", level).hash(&mut hasher);
                }
                if let Some(ref name_pattern) = filters.name_pattern {
                    name_pattern.hash(&mut hasher);
                }
                if let Some(ref color) = filters.color {
                    color.hash(&mut hasher);
                }
                if let Some(limit) = filters.limit {
                    limit.hash(&mut hasher);
                }
                if let Some(offset) = filters.offset {
                    offset.hash(&mut hasher);
                }
            }
            WebSocketCommand::BatchCreateLabels { data, .. } => {
                "batch_create_labels".hash(&mut hasher);
                data.len().hash(&mut hasher);
                for item in data {
                    item.name.hash(&mut hasher);
                    item.color.hash(&mut hasher);
                }
            }
            WebSocketCommand::BatchUpdateLabels { updates, .. } => {
                "batch_update_labels".hash(&mut hasher);
                updates.len().hash(&mut hasher);
                for update in updates {
                    update.label_id.hash(&mut hasher);
                }
            }
            WebSocketCommand::BatchDeleteLabels { label_ids, .. } => {
                "batch_delete_labels".hash(&mut hasher);
                label_ids.len().hash(&mut hasher);
                for label_id in label_ids {
                    label_id.hash(&mut hasher);
                }
            }
            WebSocketCommand::Subscribe { topics, .. } => {
                "subscribe".hash(&mut hasher);
                topics.len().hash(&mut hasher);
                for topic in topics {
                    topic.hash(&mut hasher);
                }
            }
            WebSocketCommand::Unsubscribe { topics, .. } => {
                "unsubscribe".hash(&mut hasher);
                topics.len().hash(&mut hasher);
                for topic in topics {
                    topic.hash(&mut hasher);
                }
            }
            WebSocketCommand::GetConnectionInfo { .. } => {
                "get_connection_info".hash(&mut hasher);
            }
            WebSocketCommand::Ping { .. } => {
                "ping".hash(&mut hasher);
            }
        }

        // 添加时间窗口（5分钟）以确保短期幂等性
        let time_window = Utc::now().timestamp() / 300; // 5分钟窗口
        time_window.hash(&mut hasher);

        format!("ws_cmd_{:x}", hasher.finish())
    }

    /// 处理安全WebSocket命令
    pub async fn handle_secure_command(
        &self,
        secure_message: SecureMessage,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> WebSocketCommandResponse {
        // 验证安全消息
        if let Err(e) = self.verify_secure_message(&secure_message).await {
            return WebSocketCommandResponse::error(
                "unknown",
                &secure_message.message_id,
                None, // 安全验证失败时没有request_id
                WebSocketCommandError::system_error(&format!(
                    "Security verification failed: {}",
                    e
                )),
            );
        }

        // 解析命令
        let command: WebSocketCommand = match serde_json::from_value(secure_message.payload.clone())
        {
            Ok(cmd) => cmd,
            Err(e) => {
                return WebSocketCommandResponse::error(
                    "unknown",
                    &secure_message.message_id,
                    None, // 解析失败时没有request_id
                    WebSocketCommandError::system_error(&format!("Failed to parse command: {}", e)),
                );
            }
        };

        tracing::info!("--------------------------------");
        tracing::info!("command: {:?}", command);
        tracing::info!("--------------------------------");

        // 处理命令
        self.handle_command(command, user).await
    }

    /// 处理WebSocket命令
    pub async fn handle_command(
        &self,
        command: WebSocketCommand,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> WebSocketCommandResponse {
        // 打印命令入参
        tracing::info!("WebSocket command received: {:?}", command);
        tracing::info!(
            "WebSocket user context: user_id={}, username={}, current_workspace_id={:?}",
            user.user_id,
            user.username,
            user.current_workspace_id
        );

        // 提取 request_id（前端传入，用于跟踪）
        let request_id = match &command {
            WebSocketCommand::CreateLabel { request_id, .. } => request_id.clone(),
            WebSocketCommand::UpdateLabel { request_id, .. } => request_id.clone(),
            WebSocketCommand::DeleteLabel { request_id, .. } => request_id.clone(),
            WebSocketCommand::QueryLabels { request_id, .. } => request_id.clone(),
            WebSocketCommand::BatchCreateLabels { request_id, .. } => request_id.clone(),
            WebSocketCommand::BatchUpdateLabels { request_id, .. } => request_id.clone(),
            WebSocketCommand::BatchDeleteLabels { request_id, .. } => request_id.clone(),
            WebSocketCommand::Subscribe { request_id, .. } => request_id.clone(),
            WebSocketCommand::Unsubscribe { request_id, .. } => request_id.clone(),
            WebSocketCommand::GetConnectionInfo { request_id, .. } => request_id.clone(),
            WebSocketCommand::Ping { request_id, .. } => request_id.clone(),
        };

        // 生成幂等性key（后端生成，基于命令内容和用户信息）
        let idempotency_key = self.generate_idempotency_key(&command, user);

        // 只对写操作或确需缓存的命令启用幂等性缓存；查询等读操作跳过缓存以避免陈旧数据
        let should_use_cache = !matches!(
            command,
            WebSocketCommand::QueryLabels { .. } | WebSocketCommand::GetConnectionInfo { .. }
        );

        // 检查幂等性
        if should_use_cache {
            if let Some(cached_response) = self.idempotency.is_processed(&idempotency_key).await {
                return cached_response;
            }
        }

        // 获取命令类型
        let command_type = match &command {
            WebSocketCommand::CreateLabel { .. } => "create_label",
            WebSocketCommand::UpdateLabel { .. } => "update_label",
            WebSocketCommand::DeleteLabel { .. } => "delete_label",
            WebSocketCommand::QueryLabels { .. } => "query_labels",
            WebSocketCommand::BatchCreateLabels { .. } => "batch_create_labels",
            WebSocketCommand::BatchUpdateLabels { .. } => "batch_update_labels",
            WebSocketCommand::BatchDeleteLabels { .. } => "batch_delete_labels",
            WebSocketCommand::Subscribe { .. } => "subscribe",
            WebSocketCommand::Unsubscribe { .. } => "unsubscribe",
            WebSocketCommand::GetConnectionInfo { .. } => "get_connection_info",
            WebSocketCommand::Ping { .. } => "ping",
        };

        // 验证用户有工作区
        let workspace_id = match user.current_workspace_id {
            Some(ws_id) => ws_id,
            None => {
                let error_response = WebSocketCommandResponse::error(
                    &command_type,
                    &idempotency_key,
                    request_id.clone(),
                    WebSocketCommandError::business_error(
                        "NO_WORKSPACE",
                        "No current workspace selected",
                    ),
                );
                self.idempotency
                    .mark_processed(idempotency_key, error_response.clone())
                    .await;
                return error_response;
            }
        };

        // 创建请求上下文
        let ctx = RequestContext {
            user_id: user.user_id,
            workspace_id,
            idempotency_key: Some(idempotency_key.clone()),
        };

        tracing::info!(
            "Request context: user_id={}, workspace_id={}, idempotency_key={}",
            ctx.user_id,
            ctx.workspace_id,
            idempotency_key
        );

        // 处理具体命令
        let result = match command {
            WebSocketCommand::CreateLabel { data, .. } => self.handle_create_label(ctx, data).await,
            WebSocketCommand::UpdateLabel { label_id, data, .. } => {
                self.handle_update_label(ctx, label_id, data).await
            }
            WebSocketCommand::DeleteLabel { label_id, .. } => {
                self.handle_delete_label(ctx, label_id).await
            }
            WebSocketCommand::QueryLabels { filters, .. } => {
                self.handle_query_labels(ctx, filters).await
            }
            WebSocketCommand::BatchCreateLabels { data, .. } => {
                self.handle_batch_create_labels(ctx, data).await
            }
            WebSocketCommand::BatchUpdateLabels { updates, .. } => {
                self.handle_batch_update_labels(ctx, updates).await
            }
            WebSocketCommand::BatchDeleteLabels { label_ids, .. } => {
                self.handle_batch_delete_labels(ctx, label_ids).await
            }
            WebSocketCommand::Subscribe { topics, .. } => self.handle_subscribe(ctx, topics).await,
            WebSocketCommand::Unsubscribe { topics, .. } => {
                self.handle_unsubscribe(ctx, topics).await
            }
            WebSocketCommand::GetConnectionInfo { .. } => {
                self.handle_get_connection_info(ctx, user).await
            }
            WebSocketCommand::Ping { .. } => Ok(serde_json::json!({"message": "pong"})),
        };

        // 构造响应
        let response = match result {
            Ok(data) => {
                tracing::info!("Command executed successfully - response data: {:?}", data);
                WebSocketCommandResponse::success(
                    &command_type,
                    &idempotency_key,
                    request_id.clone(),
                    data,
                )
            }
            Err(app_error) => {
                tracing::error!("Command execution failed - error: {:?}", app_error);
                WebSocketCommandResponse::error(
                    &command_type,
                    &idempotency_key,
                    request_id.clone(),
                    WebSocketCommandError::business_error("COMMAND_ERROR", &app_error.to_string()),
                )
            }
        };

        // 缓存响应（仅在启用缓存时）
        if should_use_cache {
            self.idempotency
                .mark_processed(idempotency_key, response.clone())
                .await;
        }
        response
    }

    /// 处理创建标签命令
    async fn handle_create_label(
        &self,
        ctx: RequestContext,
        data: CreateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        tracing::info!("CreateLabel command - data: {:?}", data);
        // 验证输入
        validate_create_label(&data.name, &data.color)?;

        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let req = crate::routes::labels::CreateLabelRequest {
            name: data.name,
            color: data.color,
            level: data.level,
        };

        let label = crate::services::labels_service::LabelsService::create(&mut conn, &ctx, &req)?;
        Ok(serde_json::to_value(&label).unwrap())
    }

    /// 处理更新标签命令
    async fn handle_update_label(
        &self,
        ctx: RequestContext,
        label_id: Uuid,
        data: UpdateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        tracing::info!(
            "UpdateLabel command - label_id: {}, data: {:?}",
            label_id,
            data
        );
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let req = crate::routes::labels::UpdateLabelRequest {
            name: data.name,
            color: data.color,
            level: data.level,
        };

        let updated = crate::services::labels_service::LabelsService::update(
            &mut conn, &ctx, label_id, &req,
        )?;
        Ok(serde_json::to_value(&updated).unwrap())
    }

    /// 处理删除标签命令
    async fn handle_delete_label(
        &self,
        ctx: RequestContext,
        label_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        tracing::info!("DeleteLabel command - label_id: {}", label_id);
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        crate::services::labels_service::LabelsService::delete(&mut conn, &ctx, label_id)?;
        Ok(serde_json::json!({"deleted": true, "label_id": label_id}))
    }

    /// 启动清理任务
    pub async fn start_cleanup_task(&self) {
        let idempotency = self.idempotency.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // 每分钟清理一次
            loop {
                interval.tick().await;
                idempotency.cleanup_expired().await;
            }
        });
    }

    /// 处理查询标签命令
    async fn handle_query_labels(
        &self,
        ctx: RequestContext,
        filters: LabelFilters,
    ) -> Result<serde_json::Value, AppError> {
        tracing::info!("QueryLabels command - filters: {:?}", filters);
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        let labels = crate::services::labels_service::LabelsService::list(
            &mut conn,
            &ctx,
            filters.name_pattern,
            filters.level,
        )?;

        tracing::info!("QueryLabels result - found {} labels", labels.len());
        for (i, label) in labels.iter().enumerate() {
            tracing::info!(
                "  Label {}: id={}, name={}, workspace_id={}",
                i,
                label.id,
                label.name,
                label.workspace_id
            );
        }

        Ok(serde_json::to_value(labels).unwrap())
    }

    /// 处理批量创建标签命令
    async fn handle_batch_create_labels(
        &self,
        ctx: RequestContext,
        data: Vec<CreateLabelCommand>,
    ) -> Result<serde_json::Value, AppError> {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for (index, label_data) in data.into_iter().enumerate() {
            match self.handle_create_label(ctx.clone(), label_data).await {
                Ok(result) => results.push(result),
                Err(e) => errors.push(serde_json::json!({
                    "index": index,
                    "error": e.to_string()
                })),
            }
        }

        Ok(serde_json::json!({
            "created": results,
            "errors": errors,
            "total_created": results.len(),
            "total_errors": errors.len()
        }))
    }

    /// 处理批量更新标签命令
    async fn handle_batch_update_labels(
        &self,
        ctx: RequestContext,
        updates: Vec<LabelUpdate>,
    ) -> Result<serde_json::Value, AppError> {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for (index, update) in updates.into_iter().enumerate() {
            match self
                .handle_update_label(ctx.clone(), update.label_id, update.data)
                .await
            {
                Ok(result) => results.push(result),
                Err(e) => errors.push(serde_json::json!({
                    "index": index,
                    "label_id": update.label_id,
                    "error": e.to_string()
                })),
            }
        }

        Ok(serde_json::json!({
            "updated": results,
            "errors": errors,
            "total_updated": results.len(),
            "total_errors": errors.len()
        }))
    }

    /// 处理批量删除标签命令
    async fn handle_batch_delete_labels(
        &self,
        ctx: RequestContext,
        label_ids: Vec<Uuid>,
    ) -> Result<serde_json::Value, AppError> {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for (index, label_id) in label_ids.into_iter().enumerate() {
            match self.handle_delete_label(ctx.clone(), label_id).await {
                Ok(result) => results.push(result),
                Err(e) => errors.push(serde_json::json!({
                    "index": index,
                    "label_id": label_id,
                    "error": e.to_string()
                })),
            }
        }

        Ok(serde_json::json!({
            "deleted": results,
            "errors": errors,
            "total_deleted": results.len(),
            "total_errors": errors.len()
        }))
    }

    /// 处理订阅命令
    async fn handle_subscribe(
        &self,
        _ctx: RequestContext,
        topics: Vec<String>,
    ) -> Result<serde_json::Value, AppError> {
        // 这里应该调用WebSocketManager的订阅方法
        // 暂时返回成功响应
        Ok(serde_json::json!({
            "subscribed_topics": topics,
            "message": "Successfully subscribed to topics"
        }))
    }

    /// 处理取消订阅命令
    async fn handle_unsubscribe(
        &self,
        _ctx: RequestContext,
        topics: Vec<String>,
    ) -> Result<serde_json::Value, AppError> {
        // 这里应该调用WebSocketManager的取消订阅方法
        // 暂时返回成功响应
        Ok(serde_json::json!({
            "unsubscribed_topics": topics,
            "message": "Successfully unsubscribed from topics"
        }))
    }

    /// 处理获取连接信息命令
    async fn handle_get_connection_info(
        &self,
        _ctx: RequestContext,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> Result<serde_json::Value, AppError> {
        let connection_info = ConnectionInfo {
            user_id: user.user_id,
            username: user.username.clone(),
            connected_at: Utc::now(), // 这里应该从连接管理器获取实际时间
            last_ping: Utc::now(),    // 这里应该从连接管理器获取实际时间
            subscriptions: vec![],    // 这里应该从连接管理器获取实际订阅
            message_queue_size: 0,    // 这里应该从连接管理器获取实际队列大小
            state: "connected".to_string(),
        };

        Ok(serde_json::to_value(connection_info).map_err(|e| AppError::Internal(e.to_string()))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_label_command_serialization() {
        let command = WebSocketCommand::CreateLabel {
            data: CreateLabelCommand {
                name: "Test Label".to_string(),
                color: "#FF0000".to_string(),
                level: LabelLevel::Project,
            },
            request_id: Some("req-123".to_string()),
        };

        let json = serde_json::to_string(&command).unwrap();
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketCommand::CreateLabel { data, request_id } => {
                assert_eq!(request_id, Some("req-123".to_string()));
                assert_eq!(data.name, "Test Label");
                assert_eq!(data.color, "#FF0000");
                assert_eq!(data.level, LabelLevel::Project);
            }
            _ => panic!("Expected CreateLabel command"),
        }
    }

    #[test]
    fn test_command_response_serialization() {
        let response = WebSocketCommandResponse::success(
            "query_labels",
            "test-key",
            Some("req-123".to_string()),
            serde_json::json!({"id": "123"}),
        );

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: WebSocketCommandResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.command_type, "query_labels");
        assert_eq!(deserialized.idempotency_key, "test-key");
        assert_eq!(deserialized.request_id, Some("req-123".to_string()));
        assert!(deserialized.success);
        assert!(deserialized.data.is_some());
        assert!(deserialized.error.is_none());
    }

    #[tokio::test]
    async fn test_idempotency_control() {
        let control = IdempotencyControl::new(60);

        let response1 = WebSocketCommandResponse::success(
            "test_command",
            "test-key",
            Some("req-123".to_string()),
            serde_json::json!({"result": "first"}),
        );

        // 第一次处理
        assert!(control.is_processed("test-key").await.is_none());
        control
            .mark_processed("test-key".to_string(), response1.clone())
            .await;

        // 第二次处理应该返回缓存的结果
        let cached = control.is_processed("test-key").await.unwrap();
        assert_eq!(cached.idempotency_key, "test-key");
        assert!(cached.success);
    }
}
