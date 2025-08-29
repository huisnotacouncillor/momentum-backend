use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::models::*;
use crate::db::enums::LabelLevel;
use crate::middleware::auth::AuthUserInfo;
use crate::schema;

#[derive(Deserialize)]
pub struct LabelQuery {
    pub name: Option<String>,
    pub level: Option<LabelLevel>,
}

#[derive(Deserialize, Serialize)]
pub struct CreateLabelRequest {
    pub name: String,
    pub color: String,
    pub level: LabelLevel,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateLabelRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub level: Option<LabelLevel>,
}

// 获取标签列表
pub async fn get_labels(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Query(params): Query<LabelQuery>,
) -> impl IntoResponse {
    let workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 构建查询
    let mut query = schema::labels::table
        .filter(schema::labels::workspace_id.eq(workspace_id))
        .into_boxed();

    // 如果提供了name参数，则进行模糊搜索
    if let Some(name) = params.name {
        query = query.filter(schema::labels::name.like(format!("%{}%", name)));
    }

    // 如果提供了level参数，则进行精确匹配
    if let Some(level) = params.level {
        query = query.filter(schema::labels::level.eq(level));
    }

    // 执行查询
    let labels_result = query
        .select(Label::as_select())
        .load::<Label>(&mut conn);

    match labels_result {
        Ok(labels) => {
            let response = ApiResponse::success(labels, "Labels retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve labels");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

// 创建标签
pub async fn create_label(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateLabelRequest>,
) -> impl IntoResponse {
    let workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 创建新标签
    let new_label = NewLabel {
        workspace_id,
        name: payload.name,
        color: payload.color,
        level: payload.level,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    let insert_result = diesel::insert_into(schema::labels::table)
        .values(&new_label)
        .get_result::<Label>(&mut conn);

    match insert_result {
        Ok(label) => {
            let response = ApiResponse::created(label, "Label created successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            eprintln!("Failed to create label: {}", e);
            let response = ApiResponse::<()>::internal_error("Failed to create label");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

// 更新标签
pub async fn update_label(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(label_id): Path<Uuid>,
    Json(payload): Json<UpdateLabelRequest>,
) -> impl IntoResponse {
    let workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查标签是否存在且属于当前工作区
    let existing_label = schema::labels::table
        .filter(schema::labels::id.eq(label_id))
        .filter(schema::labels::workspace_id.eq(workspace_id))
        .select(Label::as_select())
        .first::<Label>(&mut conn);

    if existing_label.is_err() {
        let response = ApiResponse::<()>::error(
            404,
            "Label not found",
            vec![ErrorDetail {
                field: None,
                code: "LABEL_NOT_FOUND".to_string(),
                message: "Label not found or not accessible".to_string(),
            }]
        );
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    // 执行更新
    if payload.name.is_some() || payload.color.is_some() || payload.level.is_some() {
        // 构建更新查询
        let update_query = schema::labels::table.filter(schema::labels::id.eq(label_id));
        
        // 根据提供的字段执行相应的更新
        let update_result = if payload.name.is_some() && payload.color.is_some() && payload.level.is_some() {
            // 三个字段都有
            diesel::update(update_query)
                .set((
                    schema::labels::name.eq(payload.name.unwrap()),
                    schema::labels::color.eq(payload.color.unwrap()),
                    schema::labels::level.eq(payload.level.unwrap())
                ))
                .get_result::<Label>(&mut conn)
        } else if payload.name.is_some() && payload.color.is_some() {
            // 只有name和color
            diesel::update(update_query)
                .set((
                    schema::labels::name.eq(payload.name.unwrap()),
                    schema::labels::color.eq(payload.color.unwrap())
                ))
                .get_result::<Label>(&mut conn)
        } else if payload.name.is_some() && payload.level.is_some() {
            // 只有name和level
            diesel::update(update_query)
                .set((
                    schema::labels::name.eq(payload.name.unwrap()),
                    schema::labels::level.eq(payload.level.unwrap())
                ))
                .get_result::<Label>(&mut conn)
        } else if payload.color.is_some() && payload.level.is_some() {
            // 只有color和level
            diesel::update(update_query)
                .set((
                    schema::labels::color.eq(payload.color.unwrap()),
                    schema::labels::level.eq(payload.level.unwrap())
                ))
                .get_result::<Label>(&mut conn)
        } else if let Some(ref name) = payload.name {
            // 只有name
            diesel::update(update_query)
                .set(schema::labels::name.eq(name))
                .get_result::<Label>(&mut conn)
        } else if let Some(ref color) = payload.color {
            // 只有color
            diesel::update(update_query)
                .set(schema::labels::color.eq(color))
                .get_result::<Label>(&mut conn)
        } else if let Some(ref level) = payload.level {
            // 只有level
            diesel::update(update_query)
                .set(schema::labels::level.eq(level))
                .get_result::<Label>(&mut conn)
        } else {
            // 这种情况不会发生，因为我们已经检查过了
            unreachable!()
        };
        
        match update_result {
            Ok(label) => {
                let response = ApiResponse::success(label, "Label updated successfully");
                (StatusCode::OK, Json(response)).into_response()
            }
            Err(e) => {
                eprintln!("Failed to update label: {}", e);
                let response = ApiResponse::<()>::internal_error("Failed to update label");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
        }
    } else {
        // 如果没有提供任何更新字段，直接返回现有标签
        match existing_label {
            Ok(label) => {
                let response = ApiResponse::success(label, "Label retrieved successfully");
                (StatusCode::OK, Json(response)).into_response()
            }
            Err(e) => {
                eprintln!("Failed to retrieve label: {}", e);
                let response = ApiResponse::<()>::internal_error("Failed to retrieve label");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
        }
    }
}

// 删除标签
pub async fn delete_label(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(label_id): Path<Uuid>,
) -> impl IntoResponse {
    let workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查标签是否存在且属于当前工作区
    let existing_label = schema::labels::table
        .filter(schema::labels::id.eq(label_id))
        .filter(schema::labels::workspace_id.eq(workspace_id))
        .select(Label::as_select())
        .first::<Label>(&mut conn);

    if existing_label.is_err() {
        let response = ApiResponse::<()>::error(
            404,
            "Label not found",
            vec![ErrorDetail {
                field: None,
                code: "LABEL_NOT_FOUND".to_string(),
                message: "Label not found or not accessible".to_string(),
            }]
        );
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    // 执行删除
    let delete_result = diesel::delete(schema::labels::table.filter(schema::labels::id.eq(label_id)))
        .execute(&mut conn);

    match delete_result {
        Ok(_) => {
            let response = ApiResponse::<()>::ok("Label deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            eprintln!("Failed to delete label: {}", e);
            let response = ApiResponse::<()>::internal_error("Failed to delete label");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}