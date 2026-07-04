//! Middleware (spec §4) — 最小骨架
//!
//! 仅定义 trait 与 chain 框架；auth / rate_limit / logging / metrics 的真正实现留到 Step 3。
//! 切勿破坏现有 `commands/handler.rs` 的执行路径——这只是 future 的可选包装层。

use async_trait::async_trait;
use serde_json::Value;

use momentum_core::error::AppError;
use momentum_core::services::context::RequestContext;

/// 一个 "等待被分发" 的命令包。
///
/// 字段尽量少（Step 1 仅仅为 trait 自洽）：
/// - `command_type`：snake_case 字符串，与 `WebSocketCommand` 派生（`#[serde(rename_all = "snake_case")]`）。
/// - `payload`：原始 JSON。
/// - `request_id`：可空，对齐 `WebSocketCommand` 里 `Option<String>` 的设计。
#[derive(Debug, Clone)]
pub struct CommandEnvelope {
    pub command_type: &'static str,
    pub payload: Value,
    pub context: RequestContext,
    pub request_id: Option<String>,
}

/// 链式中间件：通过 `NextMiddleware` 控制调用链。
///
/// 注意：本 trait 不假设 handler 必须存在；中间件可独立运行（如 logging / metrics）。
#[async_trait]
pub trait CommandMiddleware: Send + Sync {
    fn name(&self) -> &'static str;

    async fn process(
        &self,
        envelope: CommandEnvelope,
        ctx: &MiddlewareContext,
        next: NextMiddleware<'_>,
    ) -> Result<Value, AppError>;
}

/// 中间件共享上下文（DB pool / feature flags / rate limiter holder）
#[derive(Clone)]
pub struct MiddlewareContext {
    pub feature_flags: std::sync::Arc<crate::websocket::feature_flags::FeatureFlags>,
}

/// "下一步" 的引用。`run()` 会执行链中下一个 middleware。
pub struct NextMiddleware<'a> {
    chain: &'a [Box<dyn CommandMiddleware>],
    index: usize,
    envelope: CommandEnvelope,
    ctx: &'a MiddlewareContext,
}

impl<'a> NextMiddleware<'a> {
    pub async fn run(self) -> Result<Value, AppError> {
        if self.index >= self.chain.len() {
            // 链结束：调用方负责终态（实际 handler 调度）。
            return Ok(self.envelope.payload);
        }
        let mw = &self.chain[self.index];
        let envelope = self.envelope.clone();
        let next = NextMiddleware {
            chain: self.chain,
            index: self.index + 1,
            envelope,
            ctx: self.ctx,
        };
        mw.process(next.envelope.clone(), self.ctx, next).await
    }
}

/// 中间件链（保持精简；不做 Builder 魔法）
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn CommandMiddleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self { middlewares: Vec::new() }
    }

    pub fn push<M: CommandMiddleware + 'static>(mut self, mw: M) -> Self {
        self.middlewares.push(Box::new(mw));
        self
    }

    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// 通过整个链；空链则原样返回 envelope.payload。
    pub async fn execute(
        &self,
        envelope: CommandEnvelope,
        ctx: &MiddlewareContext,
    ) -> Result<Value, AppError> {
        let next = NextMiddleware {
            chain: &self.middlewares,
            index: 0,
            envelope,
            ctx,
        };
        next.run().await
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use uuid::Uuid;

    fn make_ctx() -> (CommandEnvelope, MiddlewareContext) {
        let env = CommandEnvelope {
            command_type: "ping",
            payload: json!({}),
            context: RequestContext {
                user_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                idempotency_key: None,
            },
            request_id: Some("req-1".into()),
        };
        let ctx = MiddlewareContext {
            feature_flags: std::sync::Arc::new(
                crate::websocket::feature_flags::FeatureFlags::default(),
            ),
        };
        (env, ctx)
    }

    /// 测试中间件：包裹一层，把 next 的结果嵌入 { name, inner }
    ///
    /// 这种 "包裹" 语义对应 onion-shape 中间件：
    ///   a -> b -> c -> 终端 -> c -> b -> a
    /// 每个 mw 把 next.run() 的结果套上自己的壳（post 处理）；
    /// 这条性质对 auth/rate_limit/logging 都成立——它们都"看"env，
    /// 并不改 env。
    struct Wrapper(String);
    #[async_trait]
    impl CommandMiddleware for Wrapper {
        fn name(&self) -> &'static str { "wrapper" }
        async fn process(
            &self,
            envelope: CommandEnvelope,
            _ctx: &MiddlewareContext,
            next: NextMiddleware<'_>,
        ) -> Result<Value, AppError> {
            let inner = next.run().await?;
            Ok(json!({ "by": self.0, "inner": inner }))
        }
    }

    /// 测序：用原子计数器记录 process 调用顺序。
    ///
    /// 注意：当前 middleware 设计里 pre 对 envelope 的修改**不会**向 next 传递，
    /// 因为 `next` 已经被构造了一个独立的 envelope 副本。所以这个 Recorder：
    /// - pre：用 atomic 分配一个序号，记录成"pre 事件"
    /// - post：拿到 next 的输出 Value，把 pre+post 事件"叠加"到输出中。
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    fn reset() { COUNTER.store(0, Ordering::SeqCst); }
    fn next_seq() -> usize { COUNTER.fetch_add(1, Ordering::SeqCst) }

    struct SequenceRecorder(&'static str);
    #[async_trait]
    impl CommandMiddleware for SequenceRecorder {
        fn name(&self) -> &'static str { "seq" }
        async fn process(
            &self,
            _envelope: CommandEnvelope,
            _ctx: &MiddlewareContext,
            next: NextMiddleware<'_>,
        ) -> Result<Value, AppError> {
            // pre：分配一个序号（不修改 envelope，仅记录时序）
            let pre_seq = next_seq();
            let name = self.0;

            // 执行链中后续（含更深 middleware + 终端）
            let mut out = next.run().await?;

            // post：再次分配序号
            let post_seq = next_seq();
            // 把本次访问压到输出 __visits
            let obj = out.as_object_mut().unwrap();
            let visits = obj
                .entry("__visits".to_string())
                .or_insert_with(|| json!([]));
            visits.as_array_mut().unwrap().push(json!({
                "name": name,
                "pre_seq": pre_seq,
                "post_seq": post_seq,
            }));
            Ok(out)
        }
    }

    #[tokio::test]
    async fn empty_chain_passes_payload_through() {
        let (env, ctx) = make_ctx();
        let chain = MiddlewareChain::new();
        let out = chain.execute(env, &ctx).await.unwrap();
        assert_eq!(out, json!({}));
    }

    #[tokio::test]
    async fn single_wrapper_wraps_terminal() {
        let (env, ctx) = make_ctx();
        let chain = MiddlewareChain::new().push(Wrapper("a".into()));
        let out = chain.execute(env, &ctx).await.unwrap();
        assert_eq!(out["by"], "a");
        // 终端返回的是 envelope.payload
        assert_eq!(out["inner"], json!({}));
    }

    #[tokio::test]
    async fn wrapper_chains_outside_in() {
        let (env, ctx) = make_ctx();
        let chain = MiddlewareChain::new()
            .push(Wrapper("a".into()))
            .push(Wrapper("b".into()));
        let out = chain.execute(env, &ctx).await.unwrap();
        // a 套 b 套 终端
        assert_eq!(out["by"], "a");
        assert_eq!(out["inner"]["by"], "b");
        assert_eq!(out["inner"]["inner"], json!({}));
    }

    #[tokio::test]
    async fn visit_order_is_fifo_pre_lifo_post() {
        reset();
        let (env, ctx) = make_ctx();
        let chain = MiddlewareChain::new()
            .push(SequenceRecorder("a"))
            .push(SequenceRecorder("b"))
            .push(SequenceRecorder("c"));
        let out = chain.execute(env, &ctx).await.unwrap();
        let v = out["__visits"].as_array().unwrap();
        // pre_seq: 0,1,2 (FIFO 进栈) 名字 a,b,c
        // post_seq: 3,4,5 (顺序分配) 但它们按"内层先 post"的顺序
        // 被 append 到 __visits，所以 __visits 数组顺序是 c,b,a
        let pre: Vec<_> = v
            .iter()
            .map(|x| x["pre_seq"].as_u64().unwrap() as usize)
            .collect();
        let names: Vec<&str> = v.iter().map(|x| x["name"].as_str().unwrap()).collect();
        // 按 pre_seq 排序 -> a=0, b=1, c=2
        assert_eq!(pre, vec![2, 1, 0]); // c,b,a (reverse insertion order)
        assert_eq!(names, vec!["c", "b", "a"]);
        // 收集 post_seq
        let post: Vec<_> = v
            .iter()
            .map(|x| x["post_seq"].as_u64().unwrap() as usize)
            .collect();
        // post 由内向外运行：c 先 (3)，然后 b (4)，然后 a (5)
        assert_eq!(post, vec![3, 4, 5]);
    }
}
