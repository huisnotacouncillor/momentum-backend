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
use uuid::Uuid;

// 请求体定义
#[derive(Deserialize)]
pub struct CreateCycleRequest {
    pub team_id: Uuid,
    pub name: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
}

#[derive(Deserialize)]
pub struct UpdateCycleRequest {
    pub team_id: Option<Uuid>,
    pub name: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub status: Option<String>,
}

/// 创建 Cycle
pub async fn create_cycle(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateCycleRequest>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证 cycle 名称不为空
    if payload.name.trim().is_empty() {
        let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
            field: Some("name".to_string()),
            code: "REQUIRED".to_string(),
            message: "Cycle name is required".to_string(),
        }]);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // 验证团队属于当前工作区
    use crate::schema::teams::dsl::*;
    let team_exists = match teams
        .filter(id.eq(payload.team_id))
        .filter(workspace_id.eq(current_workspace_id))
        .select(crate::schema::teams::id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(result) => result.is_some(),
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if !team_exists {
        let response = ApiResponse::<()>::not_found("Team not found in current workspace");
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    // 创建 cycle
    let new_cycle = NewCycle {
        team_id: payload.team_id,
        name: payload.name.clone(),
        start_date: payload.start_date,
        end_date: payload.end_date,
    };

    let cycle = match diesel::insert_into(schema::cycles::table)
        .values(&new_cycle)
        .get_result::<Cycle>(&mut conn)
    {
        Ok(cycle) => cycle,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to create cycle");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(cycle),
        "Cycle created successfully",
    );
    (StatusCode::CREATED, Json(response)).into_response()
}

/// 获取 cycles 列表
pub async fn get_cycles(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 查询当前工作区下的所有 cycles（通过团队关联）
    let cycles_list = match schema::cycles::table
        .inner_join(schema::teams::table.on(
            schema::cycles::team_id.eq(schema::teams::id)
        ))
        .filter(schema::teams::workspace_id.eq(current_workspace_id))
        .select(Cycle::as_select())
        .order(schema::cycles::created_at.desc())
        .load::<Cycle>(&mut conn)
    {
        Ok(list) => list,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve cycles");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(cycles_list),
        "Cycles retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

/// 获取指定 cycle
pub async fn get_cycle_by_id(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 查询指定的 cycle 并确保它属于当前工作区
    let cycle = match schema::cycles::table
        .inner_join(schema::teams::table.on(
            schema::cycles::team_id.eq(schema::teams::id)
        ))
        .filter(schema::cycles::id.eq(cycle_id))
        .filter(schema::teams::workspace_id.eq(current_workspace_id))
        .select(Cycle::as_select())
        .first::<Cycle>(&mut conn)
        .optional()
    {
        Ok(Some(cycle)) => cycle,
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Cycle not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        },
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve cycle");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(cycle),
        "Cycle retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

/// 更新指定 cycle
pub async fn update_cycle(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
    Json(payload): Json<UpdateCycleRequest>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 如果提供了 team_id，验证团队属于当前工作区
    if let Some(team_id) = payload.team_id {
        use crate::schema::teams::dsl::*;
        let team_exists = match teams
            .filter(id.eq(team_id))
            .filter(workspace_id.eq(current_workspace_id))
            .select(crate::schema::teams::id)
            .first::<Uuid>(&mut conn)
            .optional()
        {
            Ok(result) => result.is_some(),
            Err(_) => {
                let response = ApiResponse::<()>::internal_error("Database error");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        };

        if !team_exists {
            let response = ApiResponse::<()>::not_found("Team not found in current workspace");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    }

    // 更新 cycle
    #[derive(Default)]
    struct UpdateCycle {
        team_id: Option<Uuid>,
        name: Option<String>,
        start_date: Option<chrono::NaiveDate>,
        end_date: Option<chrono::NaiveDate>,
        status: Option<String>,
    }

    let mut update_data = UpdateCycle::default();
    if let Some(team_id) = payload.team_id {
        update_data.team_id = Some(team_id);
    }
    if let Some(name) = payload.name {
        update_data.name = Some(name);
    }
    if let Some(start_date) = payload.start_date {
        update_data.start_date = Some(start_date);
    }
    if let Some(end_date) = payload.end_date {
        update_data.end_date = Some(end_date);
    }
    if let Some(status) = payload.status {
        update_data.status = Some(status);
    }

    let cycle = match diesel::update(
        schema::cycles::table
            .filter(schema::cycles::id.eq(cycle_id))
    )
    .set((
        update_data.team_id.map(|v| schema::cycles::team_id.eq(v)),
        update_data.name.map(|v| schema::cycles::name.eq(v)),
        update_data.start_date.map(|v| schema::cycles::start_date.eq(v)),
        update_data.end_date.map(|v| schema::cycles::end_date.eq(v)),
        update_data.status.map(|v| schema::cycles::status.eq(v)),
    ))
    .get_result::<Cycle>(&mut conn)
    {
        Ok(cycle) => cycle,
        Err(diesel::result::Error::NotFound) => {
            let response = ApiResponse::<()>::not_found("Cycle not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        },
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update cycle");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(cycle),
        "Cycle updated successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

/// 删除指定 cycle
pub async fn delete_cycle(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 首先检查 cycle 是否存在且属于当前工作区
    let cycle_exists = match schema::cycles::table
        .inner_join(schema::teams::table.on(
            schema::cycles::team_id.eq(schema::teams::id)
        ))
        .filter(schema::cycles::id.eq(cycle_id))
        .filter(schema::teams::workspace_id.eq(current_workspace_id))
        .select(schema::cycles::id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    if !cycle_exists {
        let response = ApiResponse::<()>::not_found("Cycle not found");
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    // 删除 cycle
    match diesel::delete(
        schema::cycles::table
            .filter(schema::cycles::id.eq(cycle_id))
    )
    .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::success(
                None as Option<String>, // 明确指定泛型类型
                "Cycle deleted successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        },
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to delete cycle");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}