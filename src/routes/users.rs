
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
    pub avatar_url: Option<String>,
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
    if payload.name.is_none() && payload.username.is_none() && payload.email.is_none() && payload.avatar_url.is_none() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: None,
            code: "NO_UPDATE_DATA".to_string(),
            message: "No update data provided".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 首先获取现有用户数据
    let existing_user: User = match schema::users::table
        .filter(schema::users::id.eq(user_id))
        .select(User::as_select())
        .first(&mut conn)
    {
        Ok(user) => user,
        Err(_) => {
            let response = ApiResponse::<()>::not_found("User not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    // 构建更新查询 - 使用提供的值或保持现有值
    let result = diesel::update(schema::users::table.filter(schema::users::id.eq(user_id)))
        .set((
            schema::users::name.eq(payload.name.unwrap_or(existing_user.name)),
            schema::users::username.eq(payload.username.unwrap_or(existing_user.username)),
            schema::users::email.eq(payload.email.unwrap_or(existing_user.email)),
            schema::users::avatar_url.eq(payload.avatar_url.or(existing_user.avatar_url)),
        ))
        .returning(User::as_returning())
        .get_result(&mut conn);

    // 处理更新结果
    let updated_user: User = match result {
        Ok(user) => user,
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) => {
            let response = ApiResponse::<()>::conflict(
                "Username or email already exists",
                None,
                "DUPLICATE_FIELD",
            );
            return (StatusCode::CONFLICT, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update user profile");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 更新缓存中的用户信息
    // 使用我们的缓存系统更新用户信息
    let redis_url = state.config.redis_url.as_str();
    if let Ok(user_cache) = crate::cache::UserCache::new(redis_url) {
        let _ = user_cache.invalidate_user_cache(user_id).await;
    }

    let response = ApiResponse::success(updated_user, "Profile updated successfully");
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn get_user(
    State(state): State<std::sync::Arc<crate::AppState>>,
    Path(user_id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    // 使用 AppState 中的资源 URL 处理工具
    let asset_helper = &state.asset_helper;

    // Check cache
    let redis_url = state.config.redis_url.as_str();
    if let Ok(user_cache) = crate::cache::UserCache::new(redis_url) {
        if let Ok(Some(cached_user)) = user_cache.get_user(user_id).await {
            let response = ApiResponse::success(cached_user, "User retrieved from cache");
            return (StatusCode::OK, Json(response)).into_response();
        }
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
    let redis_url = state.config.redis_url.as_str();
    if let Ok(user_cache) = crate::cache::UserCache::new(redis_url) {
        // 将 User 转换为 AuthUser 进行缓存
        let processed_avatar_url = user.get_processed_avatar_url(&asset_helper);
        let auth_user = AuthUser {
            id: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
            name: user.name.clone(),
            avatar_url: processed_avatar_url,
        };
        let _ = user_cache.cache_user(&auth_user).await;
    }

    // 创建处理后的用户信息
    let processed_avatar_url = user.get_processed_avatar_url(&asset_helper);
    let user_response = AuthUser {
        id: user.id,
        email: user.email,
        username: user.username,
        name: user.name,
        avatar_url: processed_avatar_url,
    };

    let response = ApiResponse::success(user_response, "User retrieved successfully");
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

    let response = ApiResponse::success(Some(users_list), "Users retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}
