//! 业务事件抽象层
//!
//! 提供业务事件的统一抽象，实现业务逻辑与WebSocket传输层的解耦

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

use super::{Event, EventContext, EventError, EventPriority, EventType, WebSocketEvent};
use crate::db::DbPool;
use crate::websocket::auth::AuthenticatedUser;

/// 业务事件特征
#[async_trait]
pub trait BusinessEvent: Send + Sync + fmt::Debug + Clone {
    type Response: Send + Sync + Serialize;
    type Error: StdError + Send + Sync;

    /// 业务事件名称
    fn event_name() -> &'static str
    where
        Self: Sized;

    /// 业务版本
    fn version() -> &'static str
    where
        Self: Sized,
    {
        "1.0"
    }

    /// 事件验证
    fn validate(&self) -> Result<(), Self::Error>;

    /// 获取关联的资源ID
    fn resource_ids(&self) -> Vec<Uuid> {
        vec![]
    }

    /// 获取权限要求
    fn required_permissions(&self) -> Vec<String> {
        vec![]
    }

    /// 是否需要事务
    fn requires_transaction(&self) -> bool {
        false
    }

    /// 获取业务标签
    fn business_tags(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// 是否可以批处理
    fn can_batch() -> bool
    where
        Self: Sized,
    {
        false
    }

    /// 幂等性键
    fn idempotency_key(&self) -> Option<String> {
        None
    }
}

/// 业务事件处理器特征
#[async_trait]
pub trait BusinessEventHandler<E: BusinessEvent>: Send + Sync {
    /// 处理业务事件
    async fn handle(&self, event: &E, ctx: &BusinessContext) -> Result<E::Response, E::Error>;

    /// 处理器名称
    fn handler_name() -> &'static str
    where
        Self: Sized;

    /// 处理器描述
    fn description(&self) -> &'static str {
        "Business event handler"
    }

    /// 预处理钩子
    async fn before_handle(&self, _event: &E, _ctx: &BusinessContext) -> Result<(), E::Error> {
        Ok(())
    }

    /// 后处理钩子
    async fn after_handle(
        &self,
        _event: &E,
        _response: &E::Response,
        _ctx: &BusinessContext,
    ) -> Result<(), E::Error> {
        Ok(())
    }

    /// 错误处理
    async fn on_error(
        &self,
        _event: &E,
        _error: &E::Error,
        _ctx: &BusinessContext,
    ) -> Option<E::Response> {
        None
    }

    /// 是否支持重试
    fn supports_retry(&self) -> bool {
        false
    }

    /// 最大重试次数
    fn max_retries(&self) -> u32 {
        3
    }
}

/// 业务上下文
#[derive(Debug, Clone)]
pub struct BusinessContext {
    /// 基础事件上下文
    pub base_context: EventContext,
    /// 数据库连接
    pub db: Arc<DbPool>,
    /// 认证用户
    pub user: AuthenticatedUser,
    /// 业务参数
    pub business_params: HashMap<String, serde_json::Value>,
    /// 事务ID
    pub transaction_id: Option<String>,
    /// 业务追踪ID
    pub trace_id: String,
    /// 租户ID
    pub tenant_id: Option<Uuid>,
}

impl BusinessContext {
    pub fn new(db: Arc<DbPool>, user: AuthenticatedUser, base_context: EventContext) -> Self {
        Self {
            base_context,
            db,
            user,
            business_params: HashMap::new(),
            transaction_id: None,
            trace_id: Uuid::new_v4().to_string(),
            tenant_id: user.current_workspace_id,
        }
    }

    /// 设置业务参数
    pub fn with_param(mut self, key: String, value: serde_json::Value) -> Self {
        self.business_params.insert(key, value);
        self
    }

    /// 获取业务参数
    pub fn get_param(&self, key: &str) -> Option<&serde_json::Value> {
        self.business_params.get(key)
    }

    /// 检查权限
    pub async fn check_permission(&self, _permission: &str) -> bool {
        // TODO: 实现具体的权限检查逻辑
        // 这里应该根据用户角色、资源权限等进行检查
        true
    }

    /// 检查资源访问权限
    pub async fn check_resource_access(&self, _resource_type: &str, _resource_id: Uuid) -> bool {
        // TODO: 实现资源访问权限检查
        true
    }

    /// 获取用户工作空间
    pub fn workspace_id(&self) -> Option<Uuid> {
        self.tenant_id
    }

    /// 创建子上下文
    pub fn create_child_context(&self, operation: &str) -> Self {
        let mut child = self.clone();
        child.trace_id = format!("{}:{}", self.trace_id, operation);
        child
    }
}

/// 通用业务事件包装器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericBusinessEvent {
    /// 事件ID
    pub id: String,
    /// 事件名称
    pub event_name: String,
    /// 事件版本
    pub version: String,
    /// 用户ID
    pub user_id: Uuid,
    /// 工作空间ID
    pub workspace_id: Option<Uuid>,
    /// 连接ID
    pub connection_id: Option<String>,
    /// 请求ID
    pub request_id: Option<String>,
    /// 事件数据
    pub payload: serde_json::Value,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 业务标签
    pub tags: HashMap<String, String>,
    /// 资源ID列表
    pub resource_ids: Vec<Uuid>,
    /// 权限要求
    pub required_permissions: Vec<String>,
    /// 幂等性键
    pub idempotency_key: Option<String>,
    /// 是否需要事务
    pub requires_transaction: bool,
    /// 优先级
    pub priority: EventPriority,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl Event for GenericBusinessEvent {
    fn event_type(&self) -> EventType {
        EventType::Business
    }

    fn priority(&self) -> EventPriority {
        self.priority.clone()
    }

    fn serialize(&self) -> Result<serde_json::Value, EventError> {
        serde_json::to_value(self).map_err(|e| EventError::SerializationError(e.to_string()))
    }

    fn event_id(&self) -> String {
        self.id.clone()
    }

    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    fn validate(&self) -> Result<(), EventError> {
        if self.event_name.is_empty() {
            return Err(EventError::ValidationError(
                "Event name cannot be empty".to_string(),
            ));
        }
        if self.payload.is_null() {
            return Err(EventError::ValidationError(
                "Event payload cannot be null".to_string(),
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl WebSocketEvent for GenericBusinessEvent {
    fn user_id(&self) -> Option<Uuid> {
        Some(self.user_id)
    }

    fn connection_id(&self) -> Option<String> {
        self.connection_id.clone()
    }

    fn workspace_id(&self) -> Option<Uuid> {
        self.workspace_id
    }

    fn should_broadcast(&self) -> bool {
        self.tags
            .get("broadcast")
            .map(|v| v == "true")
            .unwrap_or(false)
    }

    fn broadcast_targets(&self) -> Vec<Uuid> {
        self.tags
            .get("broadcast_targets")
            .and_then(|targets| serde_json::from_str(targets).ok())
            .unwrap_or_default()
    }

    fn should_persist(&self) -> bool {
        self.tags
            .get("persist")
            .map(|v| v == "true")
            .unwrap_or(false)
    }
}

/// 具体的业务事件实现

/// 标签相关事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelEvent {
    pub action: LabelAction,
    pub label_data: LabelEventData,
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelAction {
    Create,
    Update,
    Delete,
    BatchCreate,
    BatchUpdate,
    BatchDelete,
    Query,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelEventData {
    pub label_id: Option<Uuid>,
    pub name: Option<String>,
    pub color: Option<String>,
    pub level: Option<String>,
    pub labels: Option<Vec<serde_json::Value>>,
    pub filters: Option<HashMap<String, serde_json::Value>>,
}

#[async_trait]
impl BusinessEvent for LabelEvent {
    type Response = LabelEventResponse;
    type Error = LabelEventError;

    fn event_name() -> &'static str {
        "label_event"
    }

    fn validate(&self) -> Result<(), Self::Error> {
        match &self.action {
            LabelAction::Create => {
                if self.label_data.name.is_none() {
                    return Err(LabelEventError::ValidationError(
                        "Label name is required".to_string(),
                    ));
                }
            }
            LabelAction::Update => {
                if self.label_data.label_id.is_none() {
                    return Err(LabelEventError::ValidationError(
                        "Label ID is required for update".to_string(),
                    ));
                }
            }
            LabelAction::Delete => {
                if self.label_data.label_id.is_none() {
                    return Err(LabelEventError::ValidationError(
                        "Label ID is required for delete".to_string(),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn resource_ids(&self) -> Vec<Uuid> {
        let mut ids = vec![self.workspace_id];
        if let Some(label_id) = self.label_data.label_id {
            ids.push(label_id);
        }
        ids
    }

    fn required_permissions(&self) -> Vec<String> {
        match &self.action {
            LabelAction::Create | LabelAction::BatchCreate => vec!["label:create".to_string()],
            LabelAction::Update | LabelAction::BatchUpdate => vec!["label:update".to_string()],
            LabelAction::Delete | LabelAction::BatchDelete => vec!["label:delete".to_string()],
            LabelAction::Query => vec!["label:read".to_string()],
        }
    }

    fn requires_transaction(&self) -> bool {
        matches!(
            self.action,
            LabelAction::BatchCreate | LabelAction::BatchUpdate | LabelAction::BatchDelete
        )
    }

    fn can_batch() -> bool {
        true
    }

    fn idempotency_key(&self) -> Option<String> {
        self.request_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelEventResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub affected_count: Option<usize>,
    pub errors: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum LabelEventError {
    ValidationError(String),
    PermissionDenied(String),
    NotFound(String),
    DatabaseError(String),
    BusinessRuleViolation(String),
}

impl fmt::Display for LabelEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabelEventError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            LabelEventError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            LabelEventError::NotFound(msg) => write!(f, "Not found: {}", msg),
            LabelEventError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            LabelEventError::BusinessRuleViolation(msg) => {
                write!(f, "Business rule violation: {}", msg)
            }
        }
    }
}

impl StdError for LabelEventError {}

/// 工作流事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub action: WorkflowAction,
    pub workflow_data: WorkflowEventData,
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowAction {
    Create,
    Update,
    Delete,
    StateTransition,
    AddState,
    RemoveState,
    Query,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEventData {
    pub workflow_id: Option<Uuid>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub from_state: Option<Uuid>,
    pub to_state: Option<Uuid>,
    pub state_data: Option<serde_json::Value>,
    pub filters: Option<HashMap<String, serde_json::Value>>,
}

#[async_trait]
impl BusinessEvent for WorkflowEvent {
    type Response = WorkflowEventResponse;
    type Error = WorkflowEventError;

    fn event_name() -> &'static str {
        "workflow_event"
    }

    fn validate(&self) -> Result<(), Self::Error> {
        match &self.action {
            WorkflowAction::Create => {
                if self.workflow_data.name.is_none() {
                    return Err(WorkflowEventError::ValidationError(
                        "Workflow name is required".to_string(),
                    ));
                }
            }
            WorkflowAction::StateTransition => {
                if self.workflow_data.from_state.is_none() || self.workflow_data.to_state.is_none()
                {
                    return Err(WorkflowEventError::ValidationError(
                        "From and to states are required for transition".to_string(),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn required_permissions(&self) -> Vec<String> {
        match &self.action {
            WorkflowAction::Create => vec!["workflow:create".to_string()],
            WorkflowAction::Update => vec!["workflow:update".to_string()],
            WorkflowAction::Delete => vec!["workflow:delete".to_string()],
            WorkflowAction::StateTransition => vec!["workflow:transition".to_string()],
            WorkflowAction::AddState | WorkflowAction::RemoveState => {
                vec!["workflow:manage_states".to_string()]
            }
            WorkflowAction::Query => vec!["workflow:read".to_string()],
        }
    }

    fn requires_transaction(&self) -> bool {
        matches!(
            self.action,
            WorkflowAction::StateTransition
                | WorkflowAction::AddState
                | WorkflowAction::RemoveState
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEventResponse {
    pub success: bool,
    pub workflow_id: Option<Uuid>,
    pub state_id: Option<Uuid>,
    pub data: Option<serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum WorkflowEventError {
    ValidationError(String),
    PermissionDenied(String),
    InvalidTransition(String),
    WorkflowNotFound(String),
    StateNotFound(String),
    DatabaseError(String),
}

impl fmt::Display for WorkflowEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkflowEventError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            WorkflowEventError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            WorkflowEventError::InvalidTransition(msg) => write!(f, "Invalid transition: {}", msg),
            WorkflowEventError::WorkflowNotFound(msg) => write!(f, "Workflow not found: {}", msg),
            WorkflowEventError::StateNotFound(msg) => write!(f, "State not found: {}", msg),
            WorkflowEventError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl StdError for WorkflowEventError {}

/// 实时协作事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationEvent {
    pub action: CollaborationAction,
    pub document_id: Uuid,
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub operation: CollaborationOperation,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationAction {
    Join,
    Leave,
    Edit,
    Cursor,
    Selection,
    Comment,
    Lock,
    Unlock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationOperation {
    pub operation_type: String,
    pub position: Option<u32>,
    pub content: Option<String>,
    pub selection_range: Option<(u32, u32)>,
    pub cursor_position: Option<u32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl BusinessEvent for CollaborationEvent {
    type Response = CollaborationEventResponse;
    type Error = CollaborationEventError;

    fn event_name() -> &'static str {
        "collaboration_event"
    }

    fn validate(&self) -> Result<(), Self::Error> {
        match &self.action {
            CollaborationAction::Edit => {
                if self.operation.content.is_none() {
                    return Err(CollaborationEventError::ValidationError(
                        "Content is required for edit operation".to_string(),
                    ));
                }
            }
            CollaborationAction::Selection => {
                if self.operation.selection_range.is_none() {
                    return Err(CollaborationEventError::ValidationError(
                        "Selection range is required".to_string(),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn resource_ids(&self) -> Vec<Uuid> {
        vec![self.document_id, self.workspace_id]
    }

    fn required_permissions(&self) -> Vec<String> {
        match &self.action {
            CollaborationAction::Join | CollaborationAction::Leave => {
                vec!["document:read".to_string()]
            }
            CollaborationAction::Edit => vec!["document:write".to_string()],
            CollaborationAction::Comment => vec!["document:comment".to_string()],
            CollaborationAction::Lock | CollaborationAction::Unlock => {
                vec!["document:lock".to_string()]
            }
            _ => vec!["document:read".to_string()],
        }
    }

    fn business_tags(&self) -> HashMap<String, String> {
        let mut tags = HashMap::new();
        tags.insert("broadcast".to_string(), "true".to_string());
        tags.insert("realtime".to_string(), "true".to_string());
        tags.insert("document_id".to_string(), self.document_id.to_string());
        tags
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationEventResponse {
    pub success: bool,
    pub document_version: Option<u32>,
    pub operation_id: Option<String>,
    pub conflicts: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum CollaborationEventError {
    ValidationError(String),
    PermissionDenied(String),
    DocumentLocked(String),
    VersionConflict(String),
    DocumentNotFound(String),
    OperationFailed(String),
}

impl fmt::Display for CollaborationEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CollaborationEventError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            CollaborationEventError::PermissionDenied(msg) => {
                write!(f, "Permission denied: {}", msg)
            }
            CollaborationEventError::DocumentLocked(msg) => write!(f, "Document locked: {}", msg),
            CollaborationEventError::VersionConflict(msg) => write!(f, "Version conflict: {}", msg),
            CollaborationEventError::DocumentNotFound(msg) => {
                write!(f, "Document not found: {}", msg)
            }
            CollaborationEventError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl StdError for CollaborationEventError {}

/// 业务事件构建器
pub struct BusinessEventBuilder;

impl BusinessEventBuilder {
    /// 创建标签事件
    pub fn label_event(action: LabelAction, user_id: Uuid, workspace_id: Uuid) -> LabelEvent {
        LabelEvent {
            action,
            label_data: LabelEventData {
                label_id: None,
                name: None,
                color: None,
                level: None,
                labels: None,
                filters: None,
            },
            user_id,
            workspace_id,
            request_id: None,
        }
    }

    /// 创建工作流事件
    pub fn workflow_event(
        action: WorkflowAction,
        user_id: Uuid,
        workspace_id: Uuid,
    ) -> WorkflowEvent {
        WorkflowEvent {
            action,
            workflow_data: WorkflowEventData {
                workflow_id: None,
                name: None,
                description: None,
                from_state: None,
                to_state: None,
                state_data: None,
                filters: None,
            },
            user_id,
            workspace_id,
            request_id: None,
        }
    }

    /// 创建协作事件
    pub fn collaboration_event(
        action: CollaborationAction,
        document_id: Uuid,
        user_id: Uuid,
        workspace_id: Uuid,
    ) -> CollaborationEvent {
        CollaborationEvent {
            action,
            document_id,
            user_id,
            workspace_id,
            operation: CollaborationOperation {
                operation_type: "unknown".to_string(),
                position: None,
                content: None,
                selection_range: None,
                cursor_position: None,
                metadata: HashMap::new(),
            },
            request_id: None,
        }
    }

    /// 转换为通用业务事件
    pub fn to_generic<E: BusinessEvent>(
        event: &E,
    ) -> Result<GenericBusinessEvent, serde_json::Error> {
        let payload = serde_json::to_value(event)?;

        Ok(GenericBusinessEvent {
            id: Uuid::new_v4().to_string(),
            event_name: E::event_name().to_string(),
            version: E::version().to_string(),
            user_id: Uuid::new_v4(), // 这里应该从具体事件中获取
            workspace_id: None,
            connection_id: None,
            request_id: None,
            payload,
            timestamp: chrono::Utc::now(),
            tags: event.business_tags(),
            resource_ids: event.resource_ids(),
            required_permissions: event.required_permissions(),
            idempotency_key: event.idempotency_key(),
            requires_transaction: event.requires_transaction(),
            priority: EventPriority::Normal,
            metadata: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_event_validation() {
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        // 测试创建标签事件验证
        let mut event =
            BusinessEventBuilder::label_event(LabelAction::Create, user_id, workspace_id);
        assert!(event.validate().is_err());

        event.label_data.name = Some("Test Label".to_string());
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_workflow_event_permissions() {
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let event =
            BusinessEventBuilder::workflow_event(WorkflowAction::Create, user_id, workspace_id);
        let permissions = event.required_permissions();

        assert_eq!(permissions, vec!["workflow:create".to_string()]);
    }

    #[test]
    fn test_collaboration_event_tags() {
        let document_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let event = BusinessEventBuilder::collaboration_event(
            CollaborationAction::Edit,
            document_id,
            user_id,
            workspace_id,
        );

        let tags = event.business_tags();
        assert_eq!(tags.get("broadcast"), Some(&"true".to_string()));
        assert_eq!(tags.get("realtime"), Some(&"true".to_string()));
    }

    #[tokio::test]
    async fn test_business_context() {
        // 这个测试需要mock数据库和用户，暂时跳过具体实现
        // 在实际使用时需要提供真实的依赖
    }
}
