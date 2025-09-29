//! 高性能批处理模块
//!
//! 提供事件和消息的批处理能力，优化系统吞吐量和性能

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::websocket::events::{
    GenericWebSocketEvent,
    core::{EventDispatcher, EventResult},
    types::EventPriority,
};

/// 批处理器配置
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// 批处理大小
    pub batch_size: usize,
    /// 批处理超时时间（毫秒）
    pub batch_timeout_ms: u64,
    /// 最大并发批处理数量
    pub max_concurrent_batches: usize,
    /// 队列容量
    pub queue_capacity: usize,
    /// 是否启用优先级处理
    pub enable_priority_queue: bool,
    /// 是否启用自适应批处理
    pub enable_adaptive_batching: bool,
    /// 性能指标收集间隔（秒）
    pub metrics_interval_seconds: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            batch_timeout_ms: 100,
            max_concurrent_batches: 10,
            queue_capacity: 10000,
            enable_priority_queue: true,
            enable_adaptive_batching: true,
            metrics_interval_seconds: 60,
        }
    }
}

/// 批处理项
#[derive(Debug, Clone)]
pub struct BatchItem<T> {
    /// 项目ID
    pub id: String,
    /// 项目数据
    pub data: T,
    /// 优先级
    pub priority: EventPriority,
    /// 创建时间
    pub created_at: Instant,
    /// 过期时间
    pub expires_at: Option<Instant>,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl<T> BatchItem<T> {
    /// 创建新的批处理项
    pub fn new(id: String, data: T, priority: EventPriority) -> Self {
        Self {
            id,
            data,
            priority,
            created_at: Instant::now(),
            expires_at: None,
            retry_count: 0,
            max_retries: 3,
            metadata: HashMap::new(),
        }
    }

    /// 设置过期时间
    pub fn with_expiration(mut self, expires_at: Instant) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// 设置重试配置
    pub fn with_retry_config(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() > expires_at
        } else {
            false
        }
    }

    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// 增加重试次数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// 批处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult<R> {
    /// 批处理ID
    pub batch_id: String,
    /// 处理的项目数量
    pub processed_count: usize,
    /// 成功的项目数量
    pub successful_count: usize,
    /// 失败的项目数量
    pub failed_count: usize,
    /// 跳过的项目数量
    pub skipped_count: usize,
    /// 处理时间（毫秒）
    pub processing_time_ms: u64,
    /// 项目结果
    pub item_results: Vec<ItemResult<R>>,
    /// 错误信息
    pub errors: Vec<String>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// 项目处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemResult<R> {
    /// 项目ID
    pub item_id: String,
    /// 是否成功
    pub success: bool,
    /// 结果数据
    pub result: Option<R>,
    /// 错误信息
    pub error: Option<String>,
    /// 处理时间（毫秒）
    pub processing_time_ms: u64,
}

/// 批处理器特征
pub trait BatchProcessor<T, R>: Send + Sync {
    /// 处理单个批次
    fn process_batch<'a>(
        &'a self,
        items: Vec<BatchItem<T>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BatchResult<R>> + Send + 'a>>;

    /// 处理器名称
    fn processor_name(&self) -> &'static str;

    /// 获取最佳批处理大小
    fn optimal_batch_size(&self) -> usize {
        50
    }

    /// 是否支持并行处理
    fn supports_parallel(&self) -> bool {
        true
    }

    /// 预处理钩子
    fn before_batch<'a>(
        &'a self,
        _items: &'a [BatchItem<T>],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    /// 后处理钩子
    fn after_batch<'a>(
        &'a self,
        _result: &'a BatchResult<R>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}

/// 通用批处理管理器
pub struct BatchProcessorManager<T, R> {
    /// 配置
    config: BatchConfig,
    /// 优先级队列
    priority_queues: Arc<RwLock<HashMap<EventPriority, VecDeque<BatchItem<T>>>>>,
    /// 普通队列
    normal_queue: Arc<RwLock<VecDeque<BatchItem<T>>>>,
    /// 处理器
    processor: Arc<dyn BatchProcessor<T, R>>,
    /// 并发控制信号量
    semaphore: Arc<Semaphore>,
    /// 性能指标
    metrics: Arc<RwLock<BatchMetrics>>,
    /// 自适应配置
    adaptive_config: Arc<RwLock<AdaptiveConfig>>,
    /// 是否正在运行
    is_running: Arc<RwLock<bool>>,
}

/// 批处理性能指标
#[derive(Debug, Default, Clone)]
pub struct BatchMetrics {
    /// 总批次数
    pub total_batches: u64,
    /// 成功批次数
    pub successful_batches: u64,
    /// 失败批次数
    pub failed_batches: u64,
    /// 处理的总项目数
    pub total_items: u64,
    /// 成功处理的项目数
    pub successful_items: u64,
    /// 平均批处理时间（毫秒）
    pub average_batch_time_ms: f64,
    /// 平均批处理大小
    pub average_batch_size: f64,
    /// 队列长度
    pub queue_length: usize,
    /// 当前并发批处理数
    pub current_concurrent_batches: usize,
    /// 最大队列长度
    pub max_queue_length: usize,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// 自适应配置
#[derive(Debug, Clone)]
struct AdaptiveConfig {
    /// 当前批处理大小
    current_batch_size: usize,
    /// 当前超时时间
    current_timeout_ms: u64,
    /// 性能历史
    performance_history: VecDeque<PerformanceSnapshot>,
    /// 最后调整时间
    last_adjustment: Instant,
}

/// 性能快照
#[derive(Debug, Clone)]
struct PerformanceSnapshot {
    /// 时间戳
    timestamp: Instant,
    /// 平均批处理时间
    average_time_ms: f64,
    /// 吞吐量（项目/秒）
    throughput: f64,
    /// 队列长度
    queue_length: usize,
    /// 批处理大小
    batch_size: usize,
}

impl<T, R> BatchProcessorManager<T, R>
where
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    /// 创建新的批处理管理器
    pub fn new(config: BatchConfig, processor: Arc<dyn BatchProcessor<T, R>>) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_batches));

        let adaptive_config = AdaptiveConfig {
            current_batch_size: config.batch_size,
            current_timeout_ms: config.batch_timeout_ms,
            performance_history: VecDeque::new(),
            last_adjustment: Instant::now(),
        };

        Self {
            config: config.clone(),
            priority_queues: Arc::new(RwLock::new(HashMap::new())),
            normal_queue: Arc::new(RwLock::new(VecDeque::new())),
            processor,
            semaphore,
            metrics: Arc::new(RwLock::new(BatchMetrics::default())),
            adaptive_config: Arc::new(RwLock::new(adaptive_config)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动批处理器
    pub async fn start(&self) {
        {
            let mut running = self.is_running.write().await;
            if *running {
                warn!("Batch processor is already running");
                return;
            }
            *running = true;
        }

        info!(
            "Starting batch processor: {}",
            self.processor.processor_name()
        );

        // 启动批处理任务
        self.start_batch_processing_task().await;

        // 启动指标收集任务
        self.start_metrics_task().await;

        // 启动自适应调整任务
        if self.config.enable_adaptive_batching {
            self.start_adaptive_task().await;
        }

        // 启动清理任务
        self.start_cleanup_task().await;
    }

    /// 停止批处理器
    pub async fn stop(&self) {
        let mut running = self.is_running.write().await;
        *running = false;
        info!(
            "Stopped batch processor: {}",
            self.processor.processor_name()
        );
    }

    /// 添加项目到批处理队列
    pub async fn add_item(&self, item: BatchItem<T>) -> Result<(), String> {
        // 检查队列容量
        let queue_length = self.get_total_queue_length().await;
        if queue_length >= self.config.queue_capacity {
            return Err("Queue capacity exceeded".to_string());
        }

        // 检查项目是否过期
        if item.is_expired() {
            return Err("Item is expired".to_string());
        }

        if self.config.enable_priority_queue {
            // 添加到优先级队列
            let mut priority_queues = self.priority_queues.write().await;
            priority_queues
                .entry(item.priority.clone())
                .or_insert_with(VecDeque::new)
                .push_back(item);
        } else {
            // 添加到普通队列
            let mut normal_queue = self.normal_queue.write().await;
            normal_queue.push_back(item);
        }

        debug!(
            "Added item to batch queue, total queue length: {}",
            queue_length + 1
        );
        Ok(())
    }

    /// 获取总队列长度
    async fn get_total_queue_length(&self) -> usize {
        if self.config.enable_priority_queue {
            let priority_queues = self.priority_queues.read().await;
            priority_queues.values().map(|q| q.len()).sum()
        } else {
            let normal_queue = self.normal_queue.read().await;
            normal_queue.len()
        }
    }

    /// 从队列中获取批次
    async fn get_batch(&self) -> Vec<BatchItem<T>> {
        let adaptive_config = self.adaptive_config.read().await;
        let batch_size = adaptive_config.current_batch_size;
        drop(adaptive_config);

        let mut batch = Vec::with_capacity(batch_size);

        if self.config.enable_priority_queue {
            // 按优先级顺序获取项目
            let priorities = vec![
                EventPriority::Critical,
                EventPriority::High,
                EventPriority::Normal,
                EventPriority::Low,
            ];

            let mut priority_queues = self.priority_queues.write().await;
            for priority in priorities {
                if let Some(queue) = priority_queues.get_mut(&priority) {
                    while batch.len() < batch_size {
                        if let Some(item) = queue.pop_front() {
                            if item.is_expired() {
                                // 跳过过期项目
                                continue;
                            }
                            batch.push(item);
                        } else {
                            break;
                        }
                    }
                    if batch.len() >= batch_size {
                        break;
                    }
                }
            }
        } else {
            // 从普通队列获取
            let mut normal_queue = self.normal_queue.write().await;
            while batch.len() < batch_size {
                if let Some(item) = normal_queue.pop_front() {
                    if item.is_expired() {
                        continue;
                    }
                    batch.push(item);
                } else {
                    break;
                }
            }
        }

        batch
    }

    /// 启动批处理任务
    async fn start_batch_processing_task(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(10)); // 高频检查

            loop {
                interval.tick().await;

                // 检查是否仍在运行
                {
                    let running = manager.is_running.read().await;
                    if !*running {
                        break;
                    }
                }

                // 检查是否有足够的项目或超时
                let should_process = manager.should_process_batch().await;
                if !should_process {
                    continue;
                }

                // 获取信号量许可
                let permit = match manager.semaphore.try_acquire() {
                    Ok(permit) => permit,
                    Err(_) => {
                        // 没有可用的许可，跳过这次处理
                        continue;
                    }
                };

                let batch = manager.get_batch().await;
                if batch.is_empty() {
                    drop(permit);
                    continue;
                }

                // 处理批次
                let manager_clone = manager.clone();
                tokio::spawn(async move {
                    let _permit = permit; // 保持许可直到任务完成
                    manager_clone.process_batch_internal(batch).await;
                });
            }
        });
    }

    /// 检查是否应该处理批次
    async fn should_process_batch(&self) -> bool {
        let queue_length = self.get_total_queue_length().await;
        let adaptive_config = self.adaptive_config.read().await;
        let batch_size = adaptive_config.current_batch_size;
        let timeout_ms = adaptive_config.current_timeout_ms;
        drop(adaptive_config);

        if queue_length >= batch_size {
            return true;
        }

        // 检查超时
        if queue_length > 0 {
            // 获取最旧的项目时间
            let oldest_time = self.get_oldest_item_time().await;
            if let Some(oldest) = oldest_time {
                let elapsed = oldest.elapsed();
                if elapsed.as_millis() >= timeout_ms as u128 {
                    return true;
                }
            }
        }

        false
    }

    /// 获取最旧项目的时间
    async fn get_oldest_item_time(&self) -> Option<Instant> {
        if self.config.enable_priority_queue {
            let priority_queues = self.priority_queues.read().await;
            priority_queues
                .values()
                .filter_map(|q| q.front())
                .map(|item| item.created_at)
                .min()
        } else {
            let normal_queue = self.normal_queue.read().await;
            normal_queue.front().map(|item| item.created_at)
        }
    }

    /// 内部批处理函数
    async fn process_batch_internal(&self, items: Vec<BatchItem<T>>) {
        let batch_id = Uuid::new_v4().to_string();
        let batch_size = items.len();
        let start_time = Instant::now();

        debug!("Processing batch {} with {} items", batch_id, batch_size);

        // 更新并发计数
        {
            let mut metrics = self.metrics.write().await;
            metrics.current_concurrent_batches += 1;
        }

        // 执行预处理钩子
        if let Err(e) = self.processor.before_batch(&items).await {
            error!("Before batch hook failed: {}", e);
            self.update_metrics_after_batch(batch_size, false, start_time.elapsed())
                .await;
            return;
        }

        // 处理批次
        let result = self.processor.process_batch(items).await;
        let processing_time = start_time.elapsed();

        // 执行后处理钩子
        if let Err(e) = self.processor.after_batch(&result).await {
            error!("After batch hook failed: {}", e);
        }

        // 更新指标
        let success = result.successful_count > 0;
        self.update_metrics_after_batch(batch_size, success, processing_time)
            .await;

        info!(
            "Batch {} completed: processed={}, successful={}, failed={}, time={}ms",
            batch_id,
            result.processed_count,
            result.successful_count,
            result.failed_count,
            result.processing_time_ms
        );
    }

    /// 更新批处理后的指标
    async fn update_metrics_after_batch(
        &self,
        batch_size: usize,
        success: bool,
        duration: Duration,
    ) {
        let mut metrics = self.metrics.write().await;

        metrics.total_batches += 1;
        metrics.total_items += batch_size as u64;
        metrics.current_concurrent_batches = metrics.current_concurrent_batches.saturating_sub(1);

        if success {
            metrics.successful_batches += 1;
            metrics.successful_items += batch_size as u64;
        } else {
            metrics.failed_batches += 1;
        }

        // 更新平均批处理时间
        let current_avg = metrics.average_batch_time_ms;
        let current_time_ms = duration.as_millis() as f64;
        metrics.average_batch_time_ms = (current_avg * (metrics.total_batches - 1) as f64
            + current_time_ms)
            / metrics.total_batches as f64;

        // 更新平均批处理大小
        let current_avg_size = metrics.average_batch_size;
        metrics.average_batch_size = (current_avg_size * (metrics.total_batches - 1) as f64
            + batch_size as f64)
            / metrics.total_batches as f64;

        metrics.queue_length = self.get_total_queue_length().await;
        metrics.max_queue_length = metrics.max_queue_length.max(metrics.queue_length);
        metrics.last_updated = chrono::Utc::now();
    }

    /// 启动指标收集任务
    async fn start_metrics_task(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval =
                interval(Duration::from_secs(manager.config.metrics_interval_seconds));

            loop {
                interval.tick().await;

                // 检查是否仍在运行
                {
                    let running = manager.is_running.read().await;
                    if !*running {
                        break;
                    }
                }

                let metrics = manager.get_metrics().await;
                info!(
                    "Batch processor metrics: batches={}, items={}, avg_time={}ms, queue_len={}",
                    metrics.total_batches,
                    metrics.total_items,
                    metrics.average_batch_time_ms,
                    metrics.queue_length
                );
            }
        });
    }

    /// 启动自适应调整任务
    async fn start_adaptive_task(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // 30秒调整一次

            loop {
                interval.tick().await;

                // 检查是否仍在运行
                {
                    let running = manager.is_running.read().await;
                    if !*running {
                        break;
                    }
                }

                manager.adjust_adaptive_config().await;
            }
        });
    }

    /// 调整自适应配置
    async fn adjust_adaptive_config(&self) {
        let metrics = self.get_metrics().await;
        let mut adaptive_config = self.adaptive_config.write().await;

        // 创建性能快照
        let snapshot = PerformanceSnapshot {
            timestamp: Instant::now(),
            average_time_ms: metrics.average_batch_time_ms,
            throughput: metrics.successful_items as f64 / 60.0, // 每秒处理的项目数
            queue_length: metrics.queue_length,
            batch_size: adaptive_config.current_batch_size,
        };

        adaptive_config.performance_history.push_back(snapshot);

        // 保持历史记录在合理范围内
        while adaptive_config.performance_history.len() > 10 {
            adaptive_config.performance_history.pop_front();
        }

        // 如果历史记录不足，不进行调整
        if adaptive_config.performance_history.len() < 3 {
            return;
        }

        // 分析性能趋势并调整参数
        let recent_throughput: f64 = adaptive_config
            .performance_history
            .iter()
            .rev()
            .take(3)
            .map(|s| s.throughput)
            .sum::<f64>()
            / 3.0;

        let old_throughput: f64 = adaptive_config
            .performance_history
            .iter()
            .take(3)
            .map(|s| s.throughput)
            .sum::<f64>()
            / 3.0;

        // 如果吞吐量下降且队列很长，增加批处理大小
        if recent_throughput < old_throughput * 0.95
            && metrics.queue_length > adaptive_config.current_batch_size * 2
        {
            adaptive_config.current_batch_size =
                (adaptive_config.current_batch_size * 12 / 10).min(self.config.batch_size * 2);
            info!(
                "Increased batch size to {}",
                adaptive_config.current_batch_size
            );
        }
        // 如果吞吐量提升且平均时间过长，减少批处理大小
        else if recent_throughput > old_throughput * 1.05
            && metrics.average_batch_time_ms > 1000.0
        {
            adaptive_config.current_batch_size =
                (adaptive_config.current_batch_size * 9 / 10).max(self.config.batch_size / 2);
            info!(
                "Decreased batch size to {}",
                adaptive_config.current_batch_size
            );
        }

        // 调整超时时间
        if metrics.queue_length < adaptive_config.current_batch_size / 2 {
            // 队列较短，增加超时时间以积累更多项目
            adaptive_config.current_timeout_ms = (adaptive_config.current_timeout_ms * 11 / 10)
                .min(self.config.batch_timeout_ms * 3);
        } else if metrics.queue_length > adaptive_config.current_batch_size * 3 {
            // 队列较长，减少超时时间以加快处理
            adaptive_config.current_timeout_ms =
                (adaptive_config.current_timeout_ms * 9 / 10).max(self.config.batch_timeout_ms / 3);
        }

        adaptive_config.last_adjustment = Instant::now();
    }

    /// 启动清理任务
    async fn start_cleanup_task(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // 5分钟清理一次

            loop {
                interval.tick().await;

                // 检查是否仍在运行
                {
                    let running = manager.is_running.read().await;
                    if !*running {
                        break;
                    }
                }

                let cleaned = manager.cleanup_expired_items().await;
                if cleaned > 0 {
                    info!("Cleaned up {} expired items", cleaned);
                }
            }
        });
    }

    /// 清理过期项目
    async fn cleanup_expired_items(&self) -> usize {
        let mut cleaned_count = 0;

        if self.config.enable_priority_queue {
            let mut priority_queues = self.priority_queues.write().await;
            for queue in priority_queues.values_mut() {
                let original_len = queue.len();
                queue.retain(|item| !item.is_expired());
                cleaned_count += original_len - queue.len();
            }
        } else {
            let mut normal_queue = self.normal_queue.write().await;
            let original_len = normal_queue.len();
            normal_queue.retain(|item| !item.is_expired());
            cleaned_count += original_len - normal_queue.len();
        }

        cleaned_count
    }

    /// 获取性能指标
    pub async fn get_metrics(&self) -> BatchMetrics {
        let mut metrics = self.metrics.read().await.clone();
        metrics.queue_length = self.get_total_queue_length().await;
        metrics
    }

    /// 重置指标
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = BatchMetrics::default();

        let mut adaptive_config = self.adaptive_config.write().await;
        adaptive_config.performance_history.clear();
    }

    /// 获取队列状态
    pub async fn get_queue_status(&self) -> serde_json::Value {
        let queue_length = self.get_total_queue_length().await;
        let adaptive_config = self.adaptive_config.read().await;

        let mut queue_by_priority = HashMap::new();
        if self.config.enable_priority_queue {
            let priority_queues = self.priority_queues.read().await;
            for (priority, queue) in priority_queues.iter() {
                queue_by_priority.insert(format!("{:?}", priority), queue.len());
            }
        }

        serde_json::json!({
            "total_length": queue_length,
            "capacity": self.config.queue_capacity,
            "utilization": queue_length as f64 / self.config.queue_capacity as f64,
            "current_batch_size": adaptive_config.current_batch_size,
            "current_timeout_ms": adaptive_config.current_timeout_ms,
            "queue_by_priority": queue_by_priority,
            "timestamp": chrono::Utc::now()
        })
    }
}

impl<T, R> Clone for BatchProcessorManager<T, R>
where
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            priority_queues: self.priority_queues.clone(),
            normal_queue: self.normal_queue.clone(),
            processor: self.processor.clone(),
            semaphore: self.semaphore.clone(),
            metrics: self.metrics.clone(),
            adaptive_config: self.adaptive_config.clone(),
            is_running: self.is_running.clone(),
        }
    }
}

/// WebSocket事件批处理器实现
pub struct WebSocketEventBatchProcessor {
    event_dispatcher: Arc<EventDispatcher>,
}

impl WebSocketEventBatchProcessor {
    pub fn new(event_dispatcher: Arc<EventDispatcher>) -> Self {
        Self { event_dispatcher }
    }
}

impl BatchProcessor<GenericWebSocketEvent, EventResult> for WebSocketEventBatchProcessor {
    fn process_batch<'a>(
        &'a self,
        items: Vec<BatchItem<GenericWebSocketEvent>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BatchResult<EventResult>> + Send + 'a>>
    {
        Box::pin(async move {
            let batch_id = Uuid::new_v4().to_string();
            let start_time = Instant::now();
            let total_items = items.len();

            let mut item_results = Vec::with_capacity(total_items);
            let mut successful_count = 0;
            let mut failed_count = 0;
            let mut errors = Vec::new();

            // 并行处理事件
            let results = futures::future::join_all(items.into_iter().map(|item| {
                let dispatcher = self.event_dispatcher.clone();
                async move {
                    let item_start = Instant::now();
                    let item_id = item.id.clone();

                    match dispatcher.dispatch(item.data).await {
                        Ok(result) => ItemResult {
                            item_id,
                            success: true,
                            result: Some(result),
                            error: None,
                            processing_time_ms: item_start.elapsed().as_millis() as u64,
                        },
                        Err(e) => ItemResult {
                            item_id,
                            success: false,
                            result: None,
                            error: Some(e.to_string()),
                            processing_time_ms: item_start.elapsed().as_millis() as u64,
                        },
                    }
                }
            }))
            .await;

            // 收集结果
            for result in results {
                if result.success {
                    successful_count += 1;
                } else {
                    failed_count += 1;
                    if let Some(ref error) = result.error {
                        errors.push(error.clone());
                    }
                }
                item_results.push(result);
            }

            BatchResult {
                batch_id,
                processed_count: total_items,
                successful_count,
                failed_count,
                skipped_count: 0,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                item_results,
                errors,
                metadata: HashMap::new(),
            }
        })
    }

    fn processor_name(&self) -> &'static str {
        "websocket_event_batch_processor"
    }

    fn optimal_batch_size(&self) -> usize {
        100
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    fn before_batch<'a>(
        &'a self,
        items: &'a [BatchItem<GenericWebSocketEvent>],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            debug!("Processing batch of {} WebSocket events", items.len());
            Ok(())
        })
    }

    fn after_batch<'a>(
        &'a self,
        result: &'a BatchResult<EventResult>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            info!(
                "Completed batch processing: {}/{} successful",
                result.successful_count, result.processed_count
            );
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::sleep;

    // 测试批处理器
    struct TestBatchProcessor {
        processed_count: Arc<AtomicUsize>,
        should_fail: bool,
    }

    impl TestBatchProcessor {
        fn new(should_fail: bool) -> Self {
            Self {
                processed_count: Arc::new(AtomicUsize::new(0)),
                should_fail,
            }
        }

        fn get_processed_count(&self) -> usize {
            self.processed_count.load(Ordering::SeqCst)
        }
    }

    impl BatchProcessor<String, String> for TestBatchProcessor {
        fn process_batch<'a>(
            &'a self,
            items: Vec<BatchItem<String>>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BatchResult<String>> + Send + 'a>>
        {
            Box::pin(async move {
                let batch_id = Uuid::new_v4().to_string();
                let start_time = Instant::now();
                let total_items = items.len();

                let mut item_results = Vec::new();
                let mut successful_count = 0;
                let mut failed_count = 0;

                for item in items {
                    let item_start = Instant::now();

                    if self.should_fail {
                        item_results.push(ItemResult {
                            item_id: item.id,
                            success: false,
                            result: None,
                            error: Some("Test failure".to_string()),
                            processing_time_ms: item_start.elapsed().as_millis() as u64,
                        });
                        failed_count += 1;
                    } else {
                        item_results.push(ItemResult {
                            item_id: item.id.clone(),
                            success: true,
                            result: Some(format!("processed_{}", item.data)),
                            error: None,
                            processing_time_ms: item_start.elapsed().as_millis() as u64,
                        });
                        successful_count += 1;
                    }

                    self.processed_count.fetch_add(1, Ordering::SeqCst);
                }

                // 模拟处理时间
                tokio::time::sleep(Duration::from_millis(10)).await;

                BatchResult {
                    batch_id,
                    processed_count: total_items,
                    successful_count,
                    failed_count,
                    skipped_count: 0,
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                    item_results,
                    errors: vec![],
                    metadata: HashMap::new(),
                }
            })
        }

        fn processor_name(&self) -> &'static str {
            "test_batch_processor"
        }
    }

    #[tokio::test]
    async fn test_batch_item_creation() {
        let item = BatchItem::new(
            "test_id".to_string(),
            "test_data".to_string(),
            EventPriority::Normal,
        )
        .with_expiration(Instant::now() + Duration::from_secs(60))
        .with_retry_config(5)
        .with_metadata("key".to_string(), "value".to_string());

        assert_eq!(item.id, "test_id");
        assert_eq!(item.data, "test_data");
        assert_eq!(item.max_retries, 5);
        assert_eq!(item.metadata.get("key"), Some(&"value".to_string()));
        assert!(!item.is_expired());
        assert!(item.can_retry());
    }

    #[tokio::test]
    async fn test_batch_processor_manager() {
        let config = BatchConfig {
            batch_size: 3,
            batch_timeout_ms: 50,
            max_concurrent_batches: 2,
            queue_capacity: 100,
            enable_priority_queue: true,
            enable_adaptive_batching: false,
            metrics_interval_seconds: 1,
        };

        let processor = Arc::new(TestBatchProcessor::new(false));
        let manager = BatchProcessorManager::new(config, processor.clone());

        // 启动管理器
        manager.start().await;

        // 添加一些项目
        for i in 0..5 {
            let item = BatchItem::new(
                format!("item_{}", i),
                format!("data_{}", i),
                EventPriority::Normal,
            );
            manager.add_item(item).await.unwrap();
        }

        // 等待处理
        sleep(Duration::from_millis(200)).await;

        // 检查结果
        let metrics = manager.get_metrics().await;
        assert!(metrics.total_batches > 0);
        assert!(metrics.total_items >= 5);

        // 停止管理器
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_priority_queue() {
        let config = BatchConfig {
            batch_size: 10,
            enable_priority_queue: true,
            ..Default::default()
        };

        let processor = Arc::new(TestBatchProcessor::new(false));
        let manager = BatchProcessorManager::new(config, processor);

        // 添加不同优先级的项目
        let high_priority_item = BatchItem::new(
            "high".to_string(),
            "high_data".to_string(),
            EventPriority::High,
        );
        let low_priority_item = BatchItem::new(
            "low".to_string(),
            "low_data".to_string(),
            EventPriority::Low,
        );

        manager.add_item(low_priority_item).await.unwrap();
        manager.add_item(high_priority_item).await.unwrap();

        // 获取批次，应该优先返回高优先级项目
        let batch = manager.get_batch().await;
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].id, "high"); // 高优先级应该在前面
    }

    #[tokio::test]
    async fn test_expired_item_cleanup() {
        let config = BatchConfig::default();
        let processor = Arc::new(TestBatchProcessor::new(false));
        let manager = BatchProcessorManager::new(config, processor);

        // 添加已过期的项目
        let expired_item = BatchItem::new(
            "expired".to_string(),
            "expired_data".to_string(),
            EventPriority::Normal,
        )
        .with_expiration(Instant::now() - Duration::from_secs(1));

        let valid_item = BatchItem::new(
            "valid".to_string(),
            "valid_data".to_string(),
            EventPriority::Normal,
        );

        manager.add_item(expired_item).await.unwrap();
        manager.add_item(valid_item).await.unwrap();

        // 清理过期项目
        let cleaned = manager.cleanup_expired_items().await;
        assert_eq!(cleaned, 1);

        // 检查队列长度
        let queue_length = manager.get_total_queue_length().await;
        assert_eq!(queue_length, 1);
    }

    #[tokio::test]
    async fn test_queue_capacity() {
        let config = BatchConfig {
            queue_capacity: 2,
            ..Default::default()
        };

        let processor = Arc::new(TestBatchProcessor::new(false));
        let manager = BatchProcessorManager::new(config, processor);

        // 添加项目直到达到容量限制
        let item1 = BatchItem::new("1".to_string(), "data1".to_string(), EventPriority::Normal);
        let item2 = BatchItem::new("2".to_string(), "data2".to_string(), EventPriority::Normal);
        let item3 = BatchItem::new("3".to_string(), "data3".to_string(), EventPriority::Normal);

        assert!(manager.add_item(item1).await.is_ok());
        assert!(manager.add_item(item2).await.is_ok());
        assert!(manager.add_item(item3).await.is_err()); // 应该失败，超出容量
    }
}
