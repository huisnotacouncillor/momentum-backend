use crate::db::{DbPool, models::*};
use crate::schema::labels;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct LabelQuery {
    pub workspace_id: uuid::Uuid,
}

pub async fn get_labels(
    State(pool): State<Arc<DbPool>>,
    Query(params): Query<LabelQuery>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let labels_list = match labels::table
        .filter(labels::workspace_id.eq(params.workspace_id))
        .select(Label::as_select())
        .load(&mut conn)
    {
        Ok(labels) => labels,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let meta = ResponseMeta {
        request_id: None,
        pagination: None,
        total_count: Some(labels_list.len() as i64),
        execution_time_ms: None,
    };

    let response =
        ApiResponse::success_with_meta(labels_list, "Labels retrieved successfully", meta);
    (StatusCode::OK, Json(response)).into_response()
}
