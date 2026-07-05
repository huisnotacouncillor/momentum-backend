use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RequestContext {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    /// 幂等键（可选）
    pub idempotency_key: Option<String>,
    /// P3.3 修复：trace_id 用于跨服务追踪请求
    pub trace_id: String,
}

impl Default for RequestContext {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            workspace_id: Uuid::nil(),
            idempotency_key: None,
            trace_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl RequestContext {
    /// 创建新的请求上下文（自动生成 trace_id）
    pub fn new(user_id: Uuid, workspace_id: Uuid) -> Self {
        Self {
            user_id,
            workspace_id,
            idempotency_key: None,
            trace_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// 使用指定 trace_id 创建
    pub fn with_trace_id(user_id: Uuid, workspace_id: Uuid, trace_id: String) -> Self {
        Self {
            user_id,
            workspace_id,
            idempotency_key: None,
            trace_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context() {
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let ctx = RequestContext::new(user_id, workspace_id);

        assert_eq!(ctx.user_id, user_id);
        assert_eq!(ctx.workspace_id, workspace_id);
        assert!(ctx.idempotency_key.is_none());
        assert!(!ctx.trace_id.is_empty());
    }

    #[test]
    fn test_default_context() {
        let ctx = RequestContext::default();
        assert_eq!(ctx.user_id, Uuid::nil());
        assert!(!ctx.trace_id.is_empty());
    }

    #[test]
    fn test_with_trace_id() {
        let ctx = RequestContext::with_trace_id(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "custom-trace-123".to_string(),
        );
        assert_eq!(ctx.trace_id, "custom-trace-123");
    }
}