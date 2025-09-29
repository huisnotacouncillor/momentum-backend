//! 统一WebSocket事件系统演示
//!
//! 这个演示展示了如何使用新的统一事件系统来处理WebSocket连接、消息和业务事件
//! 包含以下功能：
//! 1. 统一事件处理架构
//! 2. 业务事件处理
//! 3. 批处理功能
//! 4. 中间件系统
//! 5. 性能监控

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use rust_backend::websocket::{
    // 认证
    AuthenticatedUser,
    BatchConfig,
    BatchItem,

    BatchProcessor,
    BatchProcessorManager,
    BroadcastMessage,
    BroadcastTarget,
    BroadcastType,
    BusinessContext,

    // 业务事件
    BusinessEvent,
    BusinessEventHandler,
    ConnectionAction,
    // 事件系统
    Event,
    EventBuilder,
    EventContext,
    EventError,
    EventHandler,
    EventMetrics,

    // 中间件
    EventMiddleware,
    // 类型
    EventPriority,
    EventResult,
    EventType,
    GenericWebSocketEvent,
    ManagerConfig,
    MessageEventType,

    MiddlewareChain,
    // 新统一系统
    UnifiedWebSocketManager,
    WebSocketEvent,
    create_default_middleware_chain,
};

/// 自定义业务事件示例：任务管理
#[derive(Debug, Clone)]
pub struct TaskEvent {
    pub task_id: Uuid,
    pub action: TaskAction,
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub task_data: TaskData,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TaskAction {
    Create,
    Update,
    Delete,
    Complete,
    Assign,
}

#[derive(Debug, Clone)]
pub struct TaskData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskEventResponse {
    pub success: bool,
    pub task_id: Uuid,
    pub message: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum TaskEventError {
    ValidationError(String),
    NotFound(String),
    PermissionDenied(String),
    DatabaseError(String),
}

impl std::fmt::Display for TaskEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskEventError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            TaskEventError::NotFound(msg) => write!(f, "Not found: {}", msg),
            TaskEventError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            TaskEventError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for TaskEventError {}

impl From<std::io::Error> for TaskEventError {
    fn from(err: std::io::Error) -> Self {
        TaskEventError::DatabaseError(err.to_string())
    }
}

// 实现业务事件特征
impl BusinessEvent for TaskEvent {
    type Response = TaskEventResponse;
    type Error = TaskEventError;

    fn event_name() -> &'static str {
        "task_event"
    }

    fn validate(&self) -> Result<(), Self::Error> {
        match &self.action {
            TaskAction::Create => {
                if self.task_data.title.is_none() {
                    return Err(TaskEventError::ValidationError(
                        "Task title is required for creation".to_string(),
                    ));
                }
            }
            TaskAction::Update => {
                // 更新操作至少需要一个字段
                if self.task_data.title.is_none()
                    && self.task_data.description.is_none()
                    && self.task_data.priority.is_none()
                    && self.task_data.assigned_to.is_none()
                    && self.task_data.due_date.is_none()
                {
                    return Err(TaskEventError::ValidationError(
                        "At least one field is required for update".to_string(),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn resource_ids(&self) -> Vec<Uuid> {
        vec![self.workspace_id, self.task_id]
    }

    fn required_permissions(&self) -> Vec<String> {
        match &self.action {
            TaskAction::Create => vec!["task:create".to_string()],
            TaskAction::Update => vec!["task:update".to_string()],
            TaskAction::Delete => vec!["task:delete".to_string()],
            TaskAction::Complete => vec!["task:complete".to_string()],
            TaskAction::Assign => vec!["task:assign".to_string()],
        }
    }

    fn requires_transaction(&self) -> bool {
        matches!(self.action, TaskAction::Delete | TaskAction::Complete)
    }

    fn business_tags(&self) -> HashMap<String, String> {
        let mut tags = HashMap::new();
        tags.insert("domain".to_string(), "task_management".to_string());
        tags.insert("action".to_string(), format!("{:?}", self.action));
        tags.insert("broadcast".to_string(), "true".to_string()); // 任务变更需要广播
        tags
    }

    fn idempotency_key(&self) -> Option<String> {
        self.request_id.clone()
    }
}

/// 任务事件处理器
pub struct TaskEventHandler;

impl TaskEventHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl BusinessEventHandler<TaskEvent> for TaskEventHandler {
    async fn handle(
        &self,
        event: &TaskEvent,
        ctx: &BusinessContext,
    ) -> Result<TaskEventResponse, TaskEventError> {
        println!(
            "🔧 Processing task event: {:?} for task {}",
            event.action, event.task_id
        );

        // 模拟数据库操作
        match &event.action {
            TaskAction::Create => {
                println!(
                    "   Creating new task: {}",
                    event.task_data.title.as_ref().unwrap()
                );
                sleep(Duration::from_millis(50)).await; // 模拟数据库写入

                Ok(TaskEventResponse {
                    success: true,
                    task_id: event.task_id,
                    message: "Task created successfully".to_string(),
                    updated_at: chrono::Utc::now(),
                })
            }
            TaskAction::Update => {
                println!("   Updating task {}", event.task_id);
                sleep(Duration::from_millis(30)).await;

                Ok(TaskEventResponse {
                    success: true,
                    task_id: event.task_id,
                    message: "Task updated successfully".to_string(),
                    updated_at: chrono::Utc::now(),
                })
            }
            TaskAction::Delete => {
                println!("   Deleting task {}", event.task_id);
                sleep(Duration::from_millis(40)).await;

                Ok(TaskEventResponse {
                    success: true,
                    task_id: event.task_id,
                    message: "Task deleted successfully".to_string(),
                    updated_at: chrono::Utc::now(),
                })
            }
            TaskAction::Complete => {
                println!("   Completing task {}", event.task_id);
                sleep(Duration::from_millis(35)).await;

                Ok(TaskEventResponse {
                    success: true,
                    task_id: event.task_id,
                    message: "Task completed successfully".to_string(),
                    updated_at: chrono::Utc::now(),
                })
            }
            TaskAction::Assign => {
                let assigned_to = event.task_data.assigned_to.unwrap_or(Uuid::new_v4());
                println!(
                    "   Assigning task {} to user {}",
                    event.task_id, assigned_to
                );
                sleep(Duration::from_millis(25)).await;

                Ok(TaskEventResponse {
                    success: true,
                    task_id: event.task_id,
                    message: format!("Task assigned to {}", assigned_to),
                    updated_at: chrono::Utc::now(),
                })
            }
        }
    }

    fn handler_name() -> &'static str {
        "task_event_handler"
    }

    async fn before_handle(
        &self,
        event: &TaskEvent,
        _ctx: &BusinessContext,
    ) -> Result<(), TaskEventError> {
        println!("   🔍 Pre-processing task event: {:?}", event.action);
        // 可以在这里进行预处理，如缓存预热、数据预加载等
        Ok(())
    }

    async fn after_handle(
        &self,
        event: &TaskEvent,
        response: &TaskEventResponse,
        _ctx: &BusinessContext,
    ) -> Result<(), TaskEventError> {
        println!(
            "   ✅ Post-processing completed for task {}: {}",
            event.task_id, response.message
        );
        // 可以在这里进行后处理，如发送通知、更新缓存等
        Ok(())
    }

    async fn on_error(
        &self,
        event: &TaskEvent,
        error: &TaskEventError,
        _ctx: &BusinessContext,
    ) -> Option<TaskEventResponse> {
        println!("   ❌ Error processing task {}: {}", event.task_id, error);
        // 可以返回默认响应或进行错误恢复
        None
    }
}

/// 自定义中间件：任务事件审计
pub struct TaskAuditMiddleware;

#[async_trait::async_trait]
impl EventMiddleware for TaskAuditMiddleware {
    fn name(&self) -> &'static str {
        "task_audit"
    }

    fn priority(&self) -> u32 {
        60 // 较低优先级，在其他中间件之后执行
    }

    async fn before_handle(
        &self,
        event: &dyn Event,
        context: &mut EventContext,
    ) -> Result<(), EventError> {
        if event.event_type() == EventType::Business {
            println!(
                "📋 Task Audit: Recording event access by user {:?}",
                context.user_id()
            );
            // 这里可以记录审计日志到数据库
        }
        Ok(())
    }

    async fn after_handle(
        &self,
        event: &dyn Event,
        context: &EventContext,
        result: &EventResult,
    ) -> Result<(), EventError> {
        if event.event_type() == EventType::Business {
            println!(
                "📋 Task Audit: Event {} completed with success={} in {}ms",
                event.event_id(),
                result.success,
                result.execution_time_ms
            );
        }
        Ok(())
    }

    fn description(&self) -> &'static str {
        "Audits task-related events for compliance"
    }

    fn should_handle(&self, event: &dyn Event) -> bool {
        event.event_type() == EventType::Business
    }
}

/// 演示统一事件系统的主要功能
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 统一WebSocket事件系统演示");
    println!("========================================");

    // 1. 演示基础事件系统
    demo_basic_event_system().await?;

    // 2. 演示业务事件处理
    demo_business_event_handling().await?;

    // 3. 演示批处理功能
    demo_batch_processing().await?;

    // 4. 演示WebSocket管理器集成
    demo_websocket_manager_integration().await?;

    // 5. 演示性能监控
    demo_performance_monitoring().await?;

    println!("\n✅ 演示完成！");
    println!("========================================");

    Ok(())
}

/// 演示基础事件系统
async fn demo_basic_event_system() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n1️⃣ 基础事件系统演示");
    println!("--------------------------------");

    // 创建基础WebSocket事件
    let user_id = Uuid::new_v4();
    let connection_id = "demo-connection-123".to_string();

    // 连接事件
    let connect_event =
        EventBuilder::connection_event(ConnectionAction::Connect, user_id, connection_id.clone());

    println!("📝 创建连接事件:");
    println!("   事件ID: {}", connect_event.event_id());
    println!("   用户ID: {}", user_id);
    println!("   连接ID: {}", connection_id);
    println!("   事件类型: {:?}", connect_event.event_type());

    // 消息事件
    let message_event = EventBuilder::message_event(
        MessageEventType::Text,
        user_id,
        None, // 广播消息
        serde_json::json!({
            "text": "Hello from unified event system!",
            "timestamp": chrono::Utc::now()
        }),
    );

    println!("\n📝 创建消息事件:");
    println!("   事件ID: {}", message_event.event_id());
    println!("   消息类型: Text");
    println!("   是否广播: {}", message_event.should_broadcast());

    Ok(())
}

/// 演示业务事件处理
async fn demo_business_event_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n2️⃣ 业务事件处理演示");
    println!("--------------------------------");

    let user_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();

    // 创建任务事件
    let create_task_event = TaskEvent {
        task_id,
        action: TaskAction::Create,
        user_id,
        workspace_id,
        task_data: TaskData {
            title: Some("实现统一事件系统".to_string()),
            description: Some("设计并实现新的统一WebSocket事件处理系统".to_string()),
            priority: Some("High".to_string()),
            assigned_to: Some(user_id),
            due_date: Some(chrono::Utc::now() + chrono::Duration::days(7)),
        },
        request_id: Some(Uuid::new_v4().to_string()),
    };

    println!("📋 创建任务事件:");
    println!("   任务ID: {}", task_id);
    println!("   操作: {:?}", create_task_event.action);
    println!(
        "   标题: {}",
        create_task_event.task_data.title.as_ref().unwrap()
    );

    // 验证事件
    match create_task_event.validate() {
        Ok(()) => println!("   ✅ 事件验证通过"),
        Err(e) => println!("   ❌ 事件验证失败: {}", e),
    }

    // 创建处理器并处理事件
    let handler = TaskEventHandler::new();

    // 模拟业务上下文（实际使用中会有真实的数据库连接）
    println!("\n🔄 处理任务事件...");

    // 这里演示事件处理逻辑
    let start_time = std::time::Instant::now();
    match handler
        .before_handle(&create_task_event, &mock_business_context())
        .await
    {
        Ok(()) => {
            match handler
                .handle(&create_task_event, &mock_business_context())
                .await
            {
                Ok(response) => {
                    let _ = handler
                        .after_handle(&create_task_event, &response, &mock_business_context())
                        .await;
                    println!("   ✅ 任务处理成功: {}", response.message);
                    println!("   处理时间: {:?}", start_time.elapsed());
                }
                Err(e) => {
                    let _ = handler
                        .on_error(&create_task_event, &e, &mock_business_context())
                        .await;
                    println!("   ❌ 任务处理失败: {}", e);
                }
            }
        }
        Err(e) => println!("   ❌ 预处理失败: {}", e),
    }

    // 演示其他任务操作
    println!("\n🔄 演示任务更新事件...");
    let update_task_event = TaskEvent {
        task_id,
        action: TaskAction::Update,
        user_id,
        workspace_id,
        task_data: TaskData {
            title: None,
            description: Some("添加更多功能细节".to_string()),
            priority: Some("Critical".to_string()),
            assigned_to: None,
            due_date: None,
        },
        request_id: Some(Uuid::new_v4().to_string()),
    };

    match handler
        .handle(&update_task_event, &mock_business_context())
        .await
    {
        Ok(response) => println!("   ✅ 任务更新成功: {}", response.message),
        Err(e) => println!("   ❌ 任务更新失败: {}", e),
    }

    Ok(())
}

/// 演示批处理功能
async fn demo_batch_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3️⃣ 批处理功能演示");
    println!("--------------------------------");

    // 创建批处理配置
    let batch_config = BatchConfig {
        batch_size: 5,
        batch_timeout_ms: 100,
        max_concurrent_batches: 2,
        queue_capacity: 50,
        enable_priority_queue: true,
        enable_adaptive_batching: false, // 为演示简化
        metrics_interval_seconds: 5,
    };

    println!("📊 批处理配置:");
    println!("   批大小: {}", batch_config.batch_size);
    println!("   超时时间: {}ms", batch_config.batch_timeout_ms);
    println!("   最大并发: {}", batch_config.max_concurrent_batches);
    println!("   启用优先级队列: {}", batch_config.enable_priority_queue);

    // 创建批处理器（演示用简化版本）
    let processor = Arc::new(DemoTaskBatchProcessor);
    let manager = BatchProcessorManager::new(batch_config, processor);

    // 启动批处理管理器
    manager.start().await;
    println!("\n🚀 批处理管理器已启动");

    // 添加批处理项目
    println!("\n➕ 添加批处理项目...");
    for i in 0..12 {
        let task_event = TaskEvent {
            task_id: Uuid::new_v4(),
            action: if i % 3 == 0 {
                TaskAction::Create
            } else {
                TaskAction::Update
            },
            user_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            task_data: TaskData {
                title: Some(format!("批处理任务 #{}", i + 1)),
                description: Some(format!("这是第 {} 个批处理任务", i + 1)),
                priority: Some("Normal".to_string()),
                assigned_to: None,
                due_date: None,
            },
            request_id: Some(Uuid::new_v4().to_string()),
        };

        let priority = match i % 4 {
            0 => EventPriority::Critical,
            1 => EventPriority::High,
            2 => EventPriority::Normal,
            _ => EventPriority::Low,
        };

        let batch_item = BatchItem::new(format!("task-{}", i + 1), task_event, priority);

        if let Err(e) = manager.add_item(batch_item).await {
            println!("   ❌ 添加项目失败: {}", e);
        } else {
            println!("   ✅ 已添加项目 #{} (优先级: {:?})", i + 1, priority);
        }
    }

    // 等待批处理完成
    println!("\n⏳ 等待批处理完成...");
    sleep(Duration::from_secs(2)).await;

    // 获取批处理指标
    let metrics = manager.get_metrics().await;
    println!("\n📊 批处理指标:");
    println!("   总批次: {}", metrics.total_batches);
    println!("   处理项目: {}", metrics.total_items);
    println!("   成功批次: {}", metrics.successful_batches);
    println!("   平均处理时间: {:.2}ms", metrics.average_batch_time_ms);
    println!("   平均批大小: {:.1}", metrics.average_batch_size);

    // 停止批处理管理器
    manager.stop().await;
    println!("\n🛑 批处理管理器已停止");

    Ok(())
}

/// 演示WebSocket管理器集成
async fn demo_websocket_manager_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n4️⃣ WebSocket管理器集成演示");
    println!("--------------------------------");

    // 创建管理器配置
    let config = ManagerConfig {
        max_connections_per_user: 5,
        message_queue_size: 1000,
        cleanup_interval_seconds: 60,
        heartbeat_interval_seconds: 30,
        connection_timeout_seconds: 300,
        message_retry_max_attempts: 3,
        event_processing_timeout_seconds: 30,
        enable_message_persistence: false,
        enable_connection_recovery: false,
    };

    println!("⚙️ 管理器配置:");
    println!("   每用户最大连接数: {}", config.max_connections_per_user);
    println!("   消息队列大小: {}", config.message_queue_size);
    println!("   清理间隔: {}秒", config.cleanup_interval_seconds);

    // 注意：这里需要真实的数据库连接，演示中我们只展示配置
    println!("\n📝 管理器功能:");
    println!("   ✅ 统一事件分发");
    println!("   ✅ 连接生命周期管理");
    println!("   ✅ 消息路由和广播");
    println!("   ✅ 性能监控");
    println!("   ✅ 中间件支持");

    // 演示广播消息结构
    let broadcast_msg = BroadcastMessage {
        id: Uuid::new_v4().to_string(),
        message_type: BroadcastType::SystemNotification,
        content: serde_json::json!({
            "title": "系统维护通知",
            "message": "系统将在30分钟后进行维护",
            "level": "warning"
        }),
        target: BroadcastTarget::All,
        created_at: chrono::Utc::now(),
        metadata: HashMap::new(),
    };

    println!("\n📢 广播消息示例:");
    println!("   消息ID: {}", broadcast_msg.id);
    println!("   类型: {:?}", broadcast_msg.message_type);
    println!("   目标: {:?}", broadcast_msg.target);

    Ok(())
}

/// 演示性能监控
async fn demo_performance_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n5️⃣ 性能监控演示");
    println!("--------------------------------");

    // 创建性能指标
    let mut metrics = EventMetrics::new();

    // 模拟一些事件处理
    println!("📊 模拟事件处理和指标收集...");

    for i in 0..20 {
        let event_type = match i % 4 {
            0 => EventType::Connection,
            1 => EventType::Message,
            2 => EventType::Business,
            _ => EventType::System,
        };

        let processing_time = 50 + (i * 10) % 200; // 模拟不同的处理时间
        let success = i % 5 != 0; // 80% 成功率

        if success {
            metrics.record_success(event_type, processing_time as u64);
        } else {
            metrics.record_failure(event_type, processing_time as u64);
        }
    }

    println!("\n📈 性能指标统计:");
    println!("   总事件数: {}", metrics.total_events);
    println!("   成功事件数: {}", metrics.successful_events);
    println!("   失败事件数: {}", metrics.failed_events);
    println!("   成功率: {:.1}%", metrics.success_rate());
    println!("   失败率: {:.1}%", metrics.failure_rate());
    println!(
        "   平均处理时间: {:.2}ms",
        metrics.average_processing_time_ms
    );
    println!("   最大处理时间: {}ms", metrics.max_processing_time_ms);
    println!("   最小处理时间: {}ms", metrics.min_processing_time_ms);

    println!("\n📊 按事件类型统计:");
    for (event_type, count) in &metrics.events_by_type {
        println!("   {:?}: {} 次", event_type, count);
    }

    // 演示中间件系统
    println!("\n🔧 中间件系统演示:");
    let mut chain = create_default_middleware_chain();

    // 添加自定义中间件
    chain.add(Arc::new(TaskAuditMiddleware));

    println!("   ✅ 已加载默认中间件链");
    println!("   ✅ 已添加任务审计中间件");

    let stats = chain.get_stats().await;
    println!("   中间件执行统计: {} 次总执行", stats.total_executions);

    Ok(())
}

/// 模拟业务上下文（实际使用中需要真实的数据库连接）
fn mock_business_context() -> BusinessContext {
    // 注意：这只是演示用的模拟上下文
    // 在实际应用中，需要提供真实的数据库连接和用户信息
    use crate::websocket::events::core::EventContext;
    use std::sync::Arc;

    let user = AuthenticatedUser {
        user_id: Uuid::new_v4(),
        username: "demo_user".to_string(),
        email: "demo@example.com".to_string(),
        name: "Demo User".to_string(),
        avatar_url: None,
        current_workspace_id: Some(Uuid::new_v4()),
    };

    // 这里应该是真实的数据库连接池
    // let db = create_mock_db_pool();

    println!("   💡 注意: 使用模拟业务上下文，实际使用需要真实数据库连接");

    // 由于无法创建真实的BusinessContext（需要数据库连接），
    // 这里只是展示结构，实际运行会需要适当的mock
    panic!("Mock context - 实际使用时请提供真实的数据库连接")
}

/// 演示用的任务批处理器
struct DemoTaskBatchProcessor;

#[async_trait::async_trait]
impl BatchProcessor<TaskEvent, TaskEventResponse> for DemoTaskBatchProcessor {
    async fn process_batch(
        &self,
        items: Vec<BatchItem<TaskEvent>>,
    ) -> rust_backend::websocket::BatchResult<TaskEventResponse> {
        use std::time::Instant;

        let batch_id = Uuid::new_v4().to_string();
        let start_time = Instant::now();
        let total_items = items.len();

        println!("   🔄 处理批次 {} ({} 个项目)", batch_id, total_items);

        let mut item_results = Vec::new();
        let mut successful_count = 0;
        let mut failed_count = 0;

        // 模拟批处理
        for (i, item) in items.into_iter().enumerate() {
            let item_start = Instant::now();

            // 模拟处理时间
            sleep(Duration::from_millis(20 + (i * 5) as u64)).await;

            // 90% 成功率
            let success = i % 10 != 0;

            if success {
                let response = TaskEventResponse {
                    success: true,
                    task_id: item.data.task_id,
                    message: format!(
                        "批处理成功: {}",
                        item.data.task_data.title.unwrap_or_default()
                    ),
                    updated_at: chrono::Utc::now(),
                };

                item_results.push(rust_backend::websocket::ItemResult {
                    item_id: item.id,
                    success: true,
                    result: Some(response),
                    error: None,
                    processing_time_ms: item_start.elapsed().as_millis() as u64,
                });

                successful_count += 1;
            } else {
                item_results.push(rust_backend::websocket::ItemResult {
                    item_id: item.id,
                    success: false,
                    result: None,
                    error: Some("模拟处理失败".to_string()),
                    processing_time_ms: item_start.elapsed().as_millis() as u64,
                });

                failed_count += 1;
            }
        }

        let result = rust_backend::websocket::BatchResult {
            batch_id,
            processed_count: total_items,
            successful_count,
            failed_count,
            skipped_count: 0,
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            item_results,
            errors: vec![],
            metadata: HashMap::new(),
        };

        println!(
            "   ✅ 批次 {} 完成: {}/{} 成功",
            batch_id, successful_count, total_items
        );
        result
    }

    fn processor_name(&self) -> &'static str {
        "demo_task_batch_processor"
    }

    fn optimal_batch_size(&self) -> usize {
        5
    }

    async fn before_batch(&self, items: &[BatchItem<TaskEvent>]) -> Result<(), String> {
        println!("   🔧 开始处理批次，包含 {} 个任务", items.len());
        Ok(())
    }

    async fn after_batch(
        &self,
        result: &rust_backend::websocket::BatchResult<TaskEventResponse>,
    ) -> Result<(), String> {
        println!(
            "   📊 批次处理完成: 成功率 {:.1}%",
            (result.successful_count as f64 / result.processed_count as f64) * 100.0
        );
        Ok(())
    }
}
