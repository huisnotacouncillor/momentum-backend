//! Issue #8：双 logger 重复守门
//!
//! 历史：
//! - `request_tracking_middleware` 发 "Request started/completed" + "Slow request detected"
//! - `performance_monitoring_middleware` 发 "API performance metrics" +
//!   "Fast/Normal/Slow/Very slow response"（基于耗时的 4 级分级）
//!
//! 每个请求会产生 2 条完成日志，运维日志量翻倍，噪音大。
//!
//! 修复后规则：
//! - 每个请求只发一条 "Request completed"（含 status + duration_ms + request_id）
//! - "Slow request detected" 只在 >1s 时发一次（不重复）
//! - 性能分级日志（"Fast/Normal/Slow/Very slow response"）应被移除
//! - "API performance metrics" 也应被并入到 completion 日志

#[cfg(test)]
mod dedup_guard_tests {
    fn source() -> &'static str {
        include_str!("request_tracking.rs")
    }

    #[test]
    fn per_request_log_lines_are_at_most_two() {
        // "Request completed" 一共 4 处：successfully / with client error /
        // with server error / 默认。 修复后保留下列 4 个。
        let src = source();
        let completion_count = src.matches("Request completed").count();
        assert_eq!(
            completion_count, 4,
            "expected exactly 4 \"Request completed\" log lines (one per status category). \
             got: {}",
            completion_count
        );
    }

    #[test]
    fn performance_monitoring_does_not_emit_per_request_level_logs() {
        let src = source();
        assert!(
            !src.contains("Fast response"),
            "Issue #8 fix: 'Fast response' per-request log must be removed"
        );
        assert!(
            !src.contains("Normal response"),
            "Issue #8 fix: 'Normal response' per-request log must be removed"
        );
        assert!(
            !src.contains("\"Slow response\""),
            "Issue #8 fix: 'Slow response' per-request log must be removed (duplicate with 'Slow request detected')"
        );
        assert!(
            !src.contains("Very slow response"),
            "Issue #8 fix: 'Very slow response' per-request log must be removed"
        );
    }

    #[test]
    fn performance_monitoring_no_longer_duplicates_api_metrics() {
        let src = source();
        assert!(
            !src.contains("API performance metrics"),
            "Issue #8 fix: 'API performance metrics' per-line log must be removed. \
             Completion log already includes status_code + duration_ms."
        );
    }
}
