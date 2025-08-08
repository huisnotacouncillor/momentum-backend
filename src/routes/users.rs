use crate::cache;
use crate::db::{DbPool, models::*};
use crate::schema;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use std::sync::Arc;

pub async fn get_user(
    State(state): State<std::sync::Arc<crate::AppState>>,
    Path(user_id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let cache_key = format!("user:{}", user_id);
    let redis = &state.redis;

    // Check cache
    if let Some(user) = cache::redis::get_cache::<User>(redis, &cache_key).await {
        let response = ApiResponse::success(user, "User retrieved from cache");
        return (StatusCode::OK, Json(response)).into_response();
    }

    // Query database
    let user: User = match schema::users::table
        .filter(schema::users::id.eq(user_id))
        .select(User::as_select())
        .first(&mut match state.db.get() {
            Ok(conn) => conn,
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Database connection failed");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }) {
        Ok(user) => user,
        Err(_) => {
            let response = ApiResponse::<()>::not_found("User not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    // Store in cache
    cache::redis::set_cache(redis, &cache_key, &user, 60).await;

    let response = ApiResponse::success(user, "User retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn get_users(State(pool): State<Arc<DbPool>>) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let users_list = match schema::users::table
        .filter(schema::users::is_active.eq(true))
        .select(User::as_select())
        .load(&mut conn)
    {
        Ok(user_list) => user_list,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let meta = ResponseMeta {
        request_id: None,
        pagination: None,
        total_count: Some(users_list.len() as i64),
        execution_time_ms: None,
    };

    let response = ApiResponse::success_with_meta(users_list, "Users retrieved successfully", meta);
    (StatusCode::OK, Json(response)).into_response()
}
