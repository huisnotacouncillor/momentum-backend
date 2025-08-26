use crate::cache;
use crate::db::{DbPool, models::*};
use crate::schema;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    TypedHeader,
};
use diesel::prelude::*;
use headers::{Authorization, authorization::Bearer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::middleware::auth::{AuthService, AuthConfig};

#[derive(Deserialize, Serialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
}

pub async fn update_profile(
    State(state): State<Arc<crate::AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer.token()) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
        }
    };

    // 检查是否提供了更新数据
    if payload.name.is_none() && payload.username.is_none() && payload.email.is_none() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: None,
            code: "NO_UPDATE_DATA".to_string(),
            message: "No update data provided".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 构建更新查询 - 需要根据提供的字段分别处理
    let result = if let Some(name) = payload.name {
        if let Some(username) = payload.username {
            if let Some(email) = payload.email {
                // 更新所有三个字段
                diesel::update(schema::users::table.filter(schema::users::id.eq(claims.sub)))
                    .set((
                        schema::users::name.eq(name),
                        schema::users::username.eq(username),
                        schema::users::email.eq(email)
                    ))
                    .returning(User::as_returning())
                    .get_result(&mut conn)
            } else {
                // 更新 name 和 username
                diesel::update(schema::users::table.filter(schema::users::id.eq(claims.sub)))
                    .set((
                        schema::users::name.eq(name),
                        schema::users::username.eq(username)
                    ))
                    .returning(User::as_returning())
                    .get_result(&mut conn)
            }
        } else if let Some(email) = payload.email {
            // 更新 name 和 email
            diesel::update(schema::users::table.filter(schema::users::id.eq(claims.sub)))
                .set((
                    schema::users::name.eq(name),
                    schema::users::email.eq(email)
                ))
                .returning(User::as_returning())
                .get_result(&mut conn)
        } else {
            // 只更新 name
            diesel::update(schema::users::table.filter(schema::users::id.eq(claims.sub)))
                .set(schema::users::name.eq(name))
                .returning(User::as_returning())
                .get_result(&mut conn)
        }
    } else if let Some(username) = payload.username {
        if let Some(email) = payload.email {
            // 更新 username 和 email
            diesel::update(schema::users::table.filter(schema::users::id.eq(claims.sub)))
                .set((
                    schema::users::username.eq(username),
                    schema::users::email.eq(email)
                ))
                .returning(User::as_returning())
                .get_result(&mut conn)
        } else {
            // 只更新 username
            diesel::update(schema::users::table.filter(schema::users::id.eq(claims.sub)))
                .set(schema::users::username.eq(username))
                .returning(User::as_returning())
                .get_result(&mut conn)
        }
    } else if let Some(email) = payload.email {
        // 只更新 email
        diesel::update(schema::users::table.filter(schema::users::id.eq(claims.sub)))
            .set(schema::users::email.eq(email))
            .returning(User::as_returning())
            .get_result(&mut conn)
    } else {
        // 不应该到达这里，但为了安全起见
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: None,
            code: "NO_UPDATE_DATA".to_string(),
            message: "No update data provided".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    };

    // 处理更新结果
    let updated_user: User = match result {
        Ok(user) => user,
        Err(diesel::result::Error::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _)) => {
            let response = ApiResponse::<()>::conflict(
                "Username or email already exists", 
                None, 
                "DUPLICATE_FIELD"
            );
            return (StatusCode::CONFLICT, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update user profile");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 更新缓存中的用户信息
    let cache_key = format!("user:{}", claims.sub);
    cache::redis::set_cache(&state.redis, &cache_key, &updated_user, 60).await;

    let response = ApiResponse::success(updated_user, "Profile updated successfully");
    (StatusCode::OK, Json(response)).into_response()
}

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