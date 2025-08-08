use crate::db::models::*;
use axum::{Json, http::StatusCode, response::IntoResponse};

pub async fn get_issue_priorities() -> impl IntoResponse {
    use crate::db::enums::IssuePriority;

    let priorities = vec![
        IssuePriority::None,
        IssuePriority::Low,
        IssuePriority::Medium,
        IssuePriority::High,
        IssuePriority::Urgent,
    ];

    let meta = ResponseMeta {
        request_id: None,
        pagination: None,
        total_count: Some(priorities.len() as i64),
        execution_time_ms: None,
    };

    let response =
        ApiResponse::success_with_meta(priorities, "Issue priorities retrieved successfully", meta);
    (StatusCode::OK, Json(response)).into_response()
}
