use crate::cache;
use crate::db::{DbPool, models::*};
use crate::middleware::auth::AuthUserInfo;
use crate::schema;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
}

pub async fn update_profile(
    State(state): State<Arc<crate::AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 使用auth_info中的用户信息，而不是重新验证token
    let user_id = auth_info.user.id;

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
                diesel::update(schema::users::table.filter(schema::users::id.eq(user_id)))
                    .set((
                        schema::users::name.eq(name),
                        schema::users::username.eq(username),
                        schema::users::email.eq(email)
                    ))
                    .returning(User::as_returning())
                    .get_result(&mut conn)
            } else {
                // 更新 name 和 username
                diesel::update(schema::users::table.filter(schema::users::id.eq(user_id)))
                    .set((
                        schema::users::name.eq(name),
                        schema::users::username.eq(username)
                    ))
                    .returning(User::as_returning())
                    .get_result(&mut conn)
            }
        } else if let Some(email) = payload.email {
            // 更新 name 和 email
            diesel::update(schema::users::table.filter(schema::users::id.eq(user_id)))
                .set((
                    schema::users::name.eq(name),
                    schema::users::email.eq(email)
                ))
                .returning(User::as_returning())
                .get_result(&mut conn)
        } else {
            // 只更新 name
            diesel::update(schema::users::table.filter(schema::users::id.eq(user_id)))
                .set(schema::users::name.eq(name))
                .returning(User::as_returning())
                .get_result(&mut conn)
        }
    } else if let Some(username) = payload.username {
        if let Some(email) = payload.email {
            // 更新 username 和 email
            diesel::update(schema::users::table.filter(schema::users::id.eq(user_id)))
                .set((
                    schema::users::username.eq(username),
                    schema::users::email.eq(email)
                ))
                .returning(User::as_returning())
                .get_result(&mut conn)
        } else {
            // 只更新 username
            diesel::update(schema::users::table.filter(schema::users::id.eq(user_id)))
                .set(schema::users::username.eq(username))
                .returning(User::as_returning())
                .get_result(&mut conn)
        }
    } else if let Some(email) = payload.email {
        // 只更新 email
        diesel::update(schema::users::table.filter(schema::users::id.eq(user_id)))
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
    let cache_key = format!("user:{}", user_id);
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

pub async fn get_users(
    State(pool): State<Arc<DbPool>>,
    _auth_info: AuthUserInfo, // 保留参数但不使用，以保持接口一致性
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    use crate::schema::users::dsl::*;
    let users_list = match users
        .filter(is_active.eq(true))
        .select(User::as_select())
        .load::<User>(&mut conn)
    {
        Ok(list) => list
            .into_iter()
            .map(|user| UserBasicInfo {
                id: user.id,
                name: user.name,
                username: user.username,
                email: user.email,
                avatar_url: user.avatar_url,
            })
            .collect::<Vec<UserBasicInfo>>(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve users");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(users_list),
        "Users retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}