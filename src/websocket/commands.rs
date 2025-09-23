use crate::{
    db::{DbPool},
    db::enums::LabelLevel,
    services::{context::RequestContext},
    validation::label::{validate_create_label, UpdateLabelChanges},
    error::AppError,
    websocket::security::SecureMessage,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// WebSocket命令类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSocketCommand {
    /// 创建标签命令
    CreateLabel {
        idempotency_key: String,
        data: CreateLabelCommand,
    },
    /// 更新标签命令
    UpdateLabel {
        idempotency_key: String,
        label_id: Uuid,
        data: UpdateLabelCommand,
    },
    /// 删除标签命令
    DeleteLabel {
        idempotency_key: String,
        label_id: Uuid,
    },
    /// 查询标签命令
    QueryLabels {
        idempotency_key: String,
        filters: LabelFilters,
    },
    /// 批量创建标签命令
    BatchCreateLabels {
        idempotency_key: String,
        data: Vec<CreateLabelCommand>,
    },
    /// 批量更新标签命令
    BatchUpdateLabels {
        idempotency_key: String,
        updates: Vec<LabelUpdate>,
    },
    /// 批量删除标签命令
    BatchDeleteLabels {
        idempotency_key: String,
        label_ids: Vec<Uuid>,
    },
    /// 订阅主题命令
    Subscribe {
        idempotency_key: String,
        topics: Vec<String>,
    },
    /// 取消订阅主题命令
    Unsubscribe {
        idempotency_key: String,
        topics: Vec<String>,
    },
    /// 获取连接信息命令
    GetConnectionInfo {
        idempotency_key: String,
    },
    /// Ping命令
    Ping {
        idempotency_key: String,
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

/// WebSocket命令响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketCommandResponse {
    pub idempotency_key: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<WebSocketCommandError>,
    pub timestamp: DateTime<Utc>,
}

/// WebSocket命令错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketCommandError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
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
    pub async fn mark_processed(&self, idempotency_key: String, response: WebSocketCommandResponse) {
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
            signer.verify_message(secure_message).await
                .map_err(|e| AppError::auth(format!("Security verification failed: {}", e)))
        } else {
            Err(AppError::Internal("Message signer not configured".to_string()))
        }
    }

    /// 处理安全WebSocket命令
    pub async fn handle_secure_command(
        &self,
        secure_message: SecureMessage,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> WebSocketCommandResponse {
        // 验证安全消息
        if let Err(e) = self.verify_secure_message(&secure_message).await {
            return WebSocketCommandResponse {
                idempotency_key: secure_message.message_id.clone(),
                success: false,
                data: None,
                error: Some(WebSocketCommandError {
                    code: "SECURITY_ERROR".to_string(),
                    message: e.to_string(),
                    field: None,
                }),
                timestamp: Utc::now(),
            };
        }

        // 解析命令
        let command: WebSocketCommand = match serde_json::from_value(secure_message.payload.clone()) {
            Ok(cmd) => cmd,
            Err(e) => {
                return WebSocketCommandResponse {
                    idempotency_key: secure_message.message_id.clone(),
                    success: false,
                    data: None,
                    error: Some(WebSocketCommandError {
                        code: "INVALID_COMMAND".to_string(),
                        message: format!("Failed to parse command: {}", e),
                        field: None,
                    }),
                    timestamp: Utc::now(),
                };
            }
        };

        // 处理命令
        self.handle_command(command, user).await
    }

    /// 处理WebSocket命令
    pub async fn handle_command(
        &self,
        command: WebSocketCommand,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> WebSocketCommandResponse {
        let idempotency_key = match &command {
            WebSocketCommand::CreateLabel { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::UpdateLabel { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::DeleteLabel { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::QueryLabels { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::BatchCreateLabels { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::BatchUpdateLabels { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::BatchDeleteLabels { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::Subscribe { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::Unsubscribe { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::GetConnectionInfo { idempotency_key, .. } => idempotency_key.clone(),
            WebSocketCommand::Ping { idempotency_key } => idempotency_key.clone(),
        };

        // 检查幂等性
        if let Some(cached_response) = self.idempotency.is_processed(&idempotency_key).await {
            return cached_response;
        }

        // 验证用户有工作区
        let workspace_id = match user.current_workspace_id {
            Some(ws_id) => ws_id,
            None => {
                let error_response = WebSocketCommandResponse {
                    idempotency_key: idempotency_key.clone(),
                    success: false,
                    data: None,
                    error: Some(WebSocketCommandError {
                        code: "NO_WORKSPACE".to_string(),
                        message: "No current workspace selected".to_string(),
                        field: None,
                    }),
                    timestamp: Utc::now(),
                };
                self.idempotency.mark_processed(idempotency_key, error_response.clone()).await;
                return error_response;
            }
        };

        // 创建请求上下文
        let ctx = RequestContext {
            user_id: user.user_id,
            workspace_id,
            idempotency_key: Some(idempotency_key.clone()),
        };

        // 处理具体命令
        let result = match command {
            WebSocketCommand::CreateLabel { data, .. } => {
                self.handle_create_label(ctx, data).await
            }
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
            WebSocketCommand::Subscribe { topics, .. } => {
                self.handle_subscribe(ctx, topics).await
            }
            WebSocketCommand::Unsubscribe { topics, .. } => {
                self.handle_unsubscribe(ctx, topics).await
            }
            WebSocketCommand::GetConnectionInfo { .. } => {
                self.handle_get_connection_info(ctx, user).await
            }
            WebSocketCommand::Ping { .. } => {
                Ok(serde_json::json!({"message": "pong"}))
            }
        };

        // 构造响应
        let response = match result {
            Ok(data) => WebSocketCommandResponse {
                idempotency_key: idempotency_key.clone(),
                success: true,
                data: Some(data),
                error: None,
                timestamp: Utc::now(),
            },
            Err(_app_error) => WebSocketCommandResponse {
                idempotency_key: idempotency_key.clone(),
                success: false,
                data: None,
                error: Some(WebSocketCommandError {
                    code: "NO_WORKSPACE".to_string(),
                    message: "No current workspace selected".to_string(),
                    field: None,
                }),
                timestamp: Utc::now(),
            },
        };

        // 缓存响应
        self.idempotency.mark_processed(idempotency_key, response.clone()).await;
        response
    }

    /// 处理创建标签命令
    async fn handle_create_label(
        &self,
        ctx: RequestContext,
        data: CreateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        // 验证输入
        validate_create_label(&data.name, &data.color)?;

        let mut conn = self.db.get().map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        // 检查名称是否已存在
        if crate::db::repositories::labels::LabelRepo::exists_by_name(&mut conn, ctx.workspace_id, &data.name)? {
            return Err(AppError::conflict_with_code(
                "Label already exists",
                Some("name".to_string()),
                "LABEL_EXISTS",
            ));
        }

        // 创建标签
        let now = chrono::Utc::now().naive_utc();
        let new_label = crate::db::models::label::NewLabel {
            workspace_id: ctx.workspace_id,
            name: data.name,
            color: data.color,
            level: data.level,
            created_at: now,
            updated_at: now,
        };

        let label = crate::db::repositories::labels::LabelRepo::insert(&mut conn, &new_label)?;
        Ok(serde_json::to_value(&label).unwrap())
    }

    /// 处理更新标签命令
    async fn handle_update_label(
        &self,
        ctx: RequestContext,
        label_id: Uuid,
        data: UpdateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = self.db.get().map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        // 确保标签存在
        let existing = crate::db::repositories::labels::LabelRepo::find_by_id_in_workspace(&mut conn, ctx.workspace_id, label_id)?;
        if existing.is_none() {
            return Err(AppError::not_found("label"));
        }

        // 验证更新数据
        let update_changes = UpdateLabelChanges {
            name: data.name.as_deref(),
            color: data.color.as_deref(),
            level_present: data.level.is_some(),
        };
        crate::validation::label::validate_update_label(&update_changes)?;

        // 检查名称唯一性
        if let Some(ref new_name) = data.name {
            if crate::db::repositories::labels::LabelRepo::exists_by_name_excluding_id(&mut conn, ctx.workspace_id, new_name, label_id)? {
                return Err(AppError::conflict_with_code(
                    "Label already exists",
                    Some("name".to_string()),
                    "LABEL_EXISTS",
                ));
            }
        }

        // 更新标签
        let updated = crate::db::repositories::labels::LabelRepo::update_fields(
            &mut conn,
            label_id,
            (data.name, data.color, data.level),
        )?;

        Ok(serde_json::to_value(&updated).unwrap())
    }

    /// 处理删除标签命令
    async fn handle_delete_label(
        &self,
        ctx: RequestContext,
        label_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = self.db.get().map_err(|_| AppError::Internal("Database connection failed".to_string()))?;

        // 确保标签存在
        let existing = crate::db::repositories::labels::LabelRepo::find_by_id_in_workspace(&mut conn, ctx.workspace_id, label_id)?;
        if existing.is_none() {
            return Err(AppError::not_found("label"));
        }

        // 删除标签
        crate::db::repositories::labels::LabelRepo::delete_by_id(&mut conn, label_id)?;
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
        _ctx: RequestContext,
        filters: LabelFilters,
    ) -> Result<serde_json::Value, AppError> {
        // 这里应该调用LabelsService的查询方法
        // 暂时返回模拟数据
        Ok(serde_json::json!({
            "labels": [],
            "total": 0,
            "filters": filters
        }))
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
            match self.handle_update_label(ctx.clone(), update.label_id, update.data).await {
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
            idempotency_key: "test-key".to_string(),
            data: CreateLabelCommand {
                name: "Test Label".to_string(),
                color: "#FF0000".to_string(),
                level: LabelLevel::Project,
            },
        };

        let json = serde_json::to_string(&command).unwrap();
        let deserialized: WebSocketCommand = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketCommand::CreateLabel { idempotency_key, data } => {
                assert_eq!(idempotency_key, "test-key");
                assert_eq!(data.name, "Test Label");
                assert_eq!(data.color, "#FF0000");
                assert_eq!(data.level, LabelLevel::Project);
            }
            _ => panic!("Expected CreateLabel command"),
        }
    }

    #[test]
    fn test_command_response_serialization() {
        let response = WebSocketCommandResponse {
            idempotency_key: "test-key".to_string(),
            success: true,
            data: Some(serde_json::json!({"id": "123"})),
            error: None,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: WebSocketCommandResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.idempotency_key, "test-key");
        assert!(deserialized.success);
        assert!(deserialized.data.is_some());
        assert!(deserialized.error.is_none());
    }

    #[tokio::test]
    async fn test_idempotency_control() {
        let control = IdempotencyControl::new(60);

        let response1 = WebSocketCommandResponse {
            idempotency_key: "test-key".to_string(),
            success: true,
            data: Some(serde_json::json!({"result": "first"})),
            error: None,
            timestamp: Utc::now(),
        };

        // 第一次处理
        assert!(control.is_processed("test-key").await.is_none());
        control.mark_processed("test-key".to_string(), response1.clone()).await;

        // 第二次处理应该返回缓存的结果
        let cached = control.is_processed("test-key").await.unwrap();
        assert_eq!(cached.idempotency_key, "test-key");
        assert!(cached.success);
    }
}
