//! Issue #12：工作区切换后必须清掉 Redis 缓存，否则用户看到旧工作区数据
//!
//! 历史：switch_workspace 路由只更新数据库，但 Redis 中 user:{id}、
//! user_profile:{id}、user_workspace:{id} 等 key 仍然指向旧工作区。
//! 直到 TTL 过期用户都看不到新数据。

#[cfg(test)]
mod switch_workspace_cache_invalidation_tests {
    /// 守门：switch_workspace handler 必须显示调用 Redis DEL 清缓存
    /// 否则用户会拿到旧工作区的缓存数据
    #[test]
    fn switch_workspace_handler_invalidates_redis_cache() {
        let source = include_str!("auth.rs");

        // 找到 switch_workspace 函数
        let start = source
            .find("pub async fn switch_workspace")
            .expect("switch_workspace handler must exist");

        // 找下一个 pub async fn 或文件末尾，以确定函数体
        let next_fn = source[start..]
            .find("\n// =====")
            .map(|i| i + start)
            .unwrap_or(source.len());
        let body = &source[start..next_fn];

        // 必须调用 Redis 清掉 user_workspace 缓存
        assert!(
            body.contains("invalidate_user_cache"),
            "switch_workspace handler MUST call invalidate_user_cache after DB update. got:\n{}",
            body
        );
    }

    /// 守门：invalidate_user_cache 帮助函数必须实际调用 .del 且包含所有 3 个 key
    #[test]
    fn invalidate_user_cache_function_actually_dels_user_workspace() {
        let source = include_str!("auth.rs");

        let start = source
            .find("async fn invalidate_user_cache")
            .expect("invalidate_user_cache helper must exist");
        // 函数体较小（< 2000 chars），直接取定长切片即可（不依赖花括号配对，
        // 因为 Rust 源码在 format!()、match 等场景里有嵌套花括号）
        let end = (start + 2500).min(source.len());
        let body = &source[start..end];

        // 必须删 user_workspace
        assert!(
            body.contains("user_workspace"),
            "must invalidate user_workspace cache key. got:\n{}",
            body
        );
        // 也清掉 user / user_profile
        assert!(
            body.contains("user_profile"),
            "must also invalidate user_profile key. got:\n{}",
            body
        );
        // 真正调用 .del(...)
        assert!(
            body.contains(".del<") || body.contains(".del::") || body.contains("del(key)"),
            "must call .del(...) on redis connection. got:\n{}",
            body
        );
    }
}
