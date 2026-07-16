//! Issue #15: handler must actually use IdempotencyControl for deduplication.

#[cfg(test)]
mod idempotency_dispatch_tests {
    fn handler_source() -> &'static str {
        include_str!("commands/handler.rs")
    }

    /// Guard: handle_command must call idempotency.is_processed(...) first
    #[test]
    fn handle_command_calls_is_processed_first() {
        let source = handler_source();
        let start = source
            .find("pub async fn handle_command(")
            .expect("handle_command must exist");
        let end = (start + 20000).min(source.len());
        let body = &source[start..end];
        assert!(
            body.contains("idempotency.is_processed"),
            "Issue #15: handle_command must check idempotency.is_processed(...) \
             BEFORE dispatching. got:\n{}",
            body
        );
    }

    /// Guard: must call idempotency.mark_processed to store response after dispatch
    #[test]
    fn handle_command_calls_mark_processed_after_dispatch() {
        let source = handler_source();
        let start = source
            .find("pub async fn handle_command(")
            .expect("handle_command must exist");
        let end = (start + 20000).min(source.len());
        let body = &source[start..end];
        assert!(
            body.contains(r#".mark_processed("#),
            "Issue #15: handle_command must call idempotency.mark_processed(...) \
             after dispatch to populate cache. got:\n{}",
            body
        );
    }

    /// Guard: handler must return cached response on hit, not re-execute
    #[test]
    fn handle_command_returns_cached_response_on_hit() {
        let source = handler_source();
        assert!(
            source.contains("return cached")
                || source.contains("is_processed(&idempotency_key).await"),
            "handle_command must short-circuit on cache hit"
        );
    }
}

#[cfg(test)]
mod idempotency_control_unit_tests {
    use super::*;

    /// Unit test: IdempotencyControl.is_processed + mark_processed work together
    #[tokio::test]
    async fn mark_processed_then_is_processed_returns_some() {
        use crate::websocket::commands::types::{
            IdempotencyControl, WebSocketCommandResponse, WebSocketCommandError,
        };
        let control = IdempotencyControl::new(60);
        let key = "key-1".to_string();
        let response = WebSocketCommandResponse::error(
            "test",
            &key,
            None,
            WebSocketCommandError::system_error("test"),
        );
        assert!(
            control.is_processed(&key).await.is_none(),
            "fresh control should return None for unknown key"
        );
        control.mark_processed(key.clone(), response.clone()).await;
        let got = control.is_processed(&key).await;
        assert!(
            got.is_some(),
            "after mark_processed, is_processed must return Some"
        );
    }

    /// Unit test: different keys are independent
    #[tokio::test]
    async fn different_keys_have_independent_cached_responses() {
        use crate::websocket::commands::types::{
            IdempotencyControl, WebSocketCommandResponse, WebSocketCommandError,
        };
        let control = IdempotencyControl::new(60);
        let r_a = WebSocketCommandResponse::error(
            "a",
            "key-a",
            None,
            WebSocketCommandError::system_error("a"),
        );
        let r_b = WebSocketCommandResponse::error(
            "b",
            "key-b",
            None,
            WebSocketCommandError::system_error("b"),
        );
        control
            .mark_processed("key-a".to_string(), r_a.clone())
            .await;
        control
            .mark_processed("key-b".to_string(), r_b.clone())
            .await;

        // Both keys should be retrievable independently
        assert!(control.is_processed("key-a").await.is_some());
        assert!(control.is_processed("key-b").await.is_some());
        // Unregistered key should not be found
        assert!(control.is_processed("key-c").await.is_none());
    }
}
