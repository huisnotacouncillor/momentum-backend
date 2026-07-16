//! Issue #14：login / register 路由成功后必须调 `RefreshTokenStore::register`
//!
//! 历史：Issue #10 添加了 RefreshTokenStore 和 /auth/refresh，但 AuthService
//! 生成的 refresh_token 从未被 register 到 store。结果：所有用户提交的
//! refresh 在 store 中都查不到，store.rotate 永远返回 Unknown。

#[cfg(test)]
mod refresh_token_registration_tests {
    /// POST /auth/login 路由在 Ok(login_response) 分支后必须 register refresh token
    #[test]
    fn login_route_registers_refresh_token_in_store() {
        let source = include_str!("auth.rs");
        let start = source
            .find("pub async fn login(")
            .expect("login route must exist");
        // 函数 body 结束：下一个 pub async fn 标志
        let end = source[start..]
            .find("\npub async fn ")
            .map(|i| i + start)
            .unwrap_or(source.len());
        let body = &source[start..end];
        // 检查两块：refresh_token_store 和 .register(...) 调用
        assert!(
            body.contains(".refresh_token_store"),
            "POST /auth/login must reach state.refresh_token_store. got:\n{}",
            body
        );
        // 在 login 函数体内应有 .register( 调用（带换行）
        assert!(
            body.contains(".register("),
            "POST /auth/login must call .register(...). got:\n{}",
            body
        );
        assert!(
            body.contains("TokenFamily"),
            "POST /auth/login must create a TokenFamily UUID per session. got:\n{}",
            body
        );
    }

    /// POST /auth/register 也必须 register
    #[test]
    fn register_route_registers_refresh_token_in_store() {
        let source = include_str!("auth.rs");
        let start = source
            .find("pub async fn register(")
            .expect("register route must exist");
        let end = source[start..]
            .find("\npub async fn login")
            .or_else(|| source[start..].find("\npub async fn "))
            .map(|i| i + start)
            .unwrap_or(source.len());
        let body = &source[start..end];
        assert!(
            body.contains(".refresh_token_store") && body.contains(".register("),
            "POST /auth/register must call state.refresh_token_store.register(...). got:\n{}",
            body
        );
    }
}
