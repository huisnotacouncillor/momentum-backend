use crate::db::{DbPool, enums::*, models::*};
use crate::middleware::auth::AuthUserInfo;
use crate::schema;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// 请求体定义
#[derive(Deserialize)]
pub struct CreateCycleRequest {
    pub team_id: Uuid,
    pub name: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub description: Option<String>,
    pub goal: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateCycleRequest {
    pub team_id: Option<Uuid>,
    pub name: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub goal: Option<String>,
}

#[derive(Deserialize)]
pub struct AssignIssuesToCycleRequest {
    pub issue_ids: Vec<Uuid>,
}

#[derive(Deserialize)]
pub struct CycleIssuesQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub status: Option<String>,
}

#[derive(Serialize)]
pub struct CycleStats {
    pub id: Uuid,
    pub name: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub status: CycleStatus,
    pub total_issues: i64,
    pub completed_issues: i64,
    pub in_progress_issues: i64,
    pub todo_issues: i64,
    pub completion_rate: f64,
    pub days_remaining: i32,
    pub is_overdue: bool,
}

#[derive(Serialize)]
pub struct CycleWithIssues {
    pub cycle: Cycle,
    pub issues: Vec<Issue>,
    pub stats: CycleStats,
}

#[derive(Serialize)]
pub struct PaginatedIssues {
    pub issues: Vec<Issue>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
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
        description: payload.description.clone(),
        goal: payload.goal.clone(),
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

    let response = ApiResponse::success(Some(cycle), "Cycle created successfully");
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
        .inner_join(schema::teams::table.on(schema::cycles::team_id.eq(schema::teams::id)))
        .filter(schema::teams::workspace_id.eq(current_workspace_id))
        .select(Cycle::as_select())
        .order(schema::cycles::start_date.desc())
        .load::<Cycle>(&mut conn)
    {
        Ok(list) => list,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve cycles");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(Some(cycles_list), "Cycles retrieved successfully");
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
        .inner_join(schema::teams::table.on(schema::cycles::team_id.eq(schema::teams::id)))
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
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve cycle");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(Some(cycle), "Cycle retrieved successfully");
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
        description: Option<String>,
        goal: Option<String>,
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
    if let Some(description) = payload.description {
        update_data.description = Some(description);
    }
    if let Some(goal) = payload.goal {
        update_data.goal = Some(goal);
    }

    let cycle = match diesel::update(schema::cycles::table.filter(schema::cycles::id.eq(cycle_id)))
        .set((
            update_data.team_id.map(|v| schema::cycles::team_id.eq(v)),
            update_data.name.map(|v| schema::cycles::name.eq(v)),
            update_data
                .start_date
                .map(|v| schema::cycles::start_date.eq(v)),
            update_data.end_date.map(|v| schema::cycles::end_date.eq(v)),
            update_data.status.map(|v| schema::cycles::status.eq(v)),
            update_data
                .description
                .map(|v| schema::cycles::description.eq(v)),
            update_data.goal.map(|v| schema::cycles::goal.eq(v)),
            Some(schema::cycles::updated_at.eq(chrono::Utc::now())),
        ))
        .get_result::<Cycle>(&mut conn)
    {
        Ok(cycle) => cycle,
        Err(diesel::result::Error::NotFound) => {
            let response = ApiResponse::<()>::not_found("Cycle not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update cycle");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(Some(cycle), "Cycle updated successfully");
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
        .inner_join(schema::teams::table.on(schema::cycles::team_id.eq(schema::teams::id)))
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
    match diesel::delete(schema::cycles::table.filter(schema::cycles::id.eq(cycle_id)))
        .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::success(
                None as Option<String>, // 明确指定泛型类型
                "Cycle deleted successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to delete cycle");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 获取周期统计信息
pub async fn get_cycle_stats(
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

    // 获取周期信息
    let cycle = match schema::cycles::table
        .inner_join(schema::teams::table.on(schema::cycles::team_id.eq(schema::teams::id)))
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
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve cycle");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 统计周期内的 issues
    let total_issues = match schema::issues::table
        .filter(schema::issues::cycle_id.eq(cycle_id))
        .count()
        .get_result::<i64>(&mut conn)
    {
        Ok(count) => count,
        Err(_) => 0,
    };

    let completed_issues = match schema::issues::table
        .left_join(
            schema::workflow_states::table
                .on(schema::issues::workflow_state_id.eq(schema::workflow_states::id.nullable())),
        )
        .filter(schema::issues::cycle_id.eq(cycle_id))
        .filter(schema::workflow_states::category.eq("done"))
        .count()
        .get_result::<i64>(&mut conn)
    {
        Ok(count) => count,
        Err(_) => 0,
    };

    let in_progress_issues = match schema::issues::table
        .left_join(
            schema::workflow_states::table
                .on(schema::issues::workflow_state_id.eq(schema::workflow_states::id.nullable())),
        )
        .filter(schema::issues::cycle_id.eq(cycle_id))
        .filter(schema::workflow_states::category.eq("started"))
        .count()
        .get_result::<i64>(&mut conn)
    {
        Ok(count) => count,
        Err(_) => 0,
    };

    let todo_issues = match schema::issues::table
        .left_join(
            schema::workflow_states::table
                .on(schema::issues::workflow_state_id.eq(schema::workflow_states::id.nullable())),
        )
        .filter(schema::issues::cycle_id.eq(cycle_id))
        .filter(schema::workflow_states::category.eq_any(vec!["unstarted", "backlog"]))
        .count()
        .get_result::<i64>(&mut conn)
    {
        Ok(count) => count,
        Err(_) => 0,
    };

    // 计算完成率
    let completion_rate = if total_issues > 0 {
        (completed_issues as f64 / total_issues as f64) * 100.0
    } else {
        0.0
    };

    // 计算剩余天数
    let today = chrono::Utc::now().date_naive();
    let days_remaining = (cycle.end_date - today).num_days() as i32;
    let is_overdue = today > cycle.end_date;

    let stats = CycleStats {
        id: cycle.id,
        name: cycle.name.clone(),
        start_date: cycle.start_date,
        end_date: cycle.end_date,
        status: cycle.status.clone(),
        total_issues,
        completed_issues,
        in_progress_issues,
        todo_issues,
        completion_rate,
        days_remaining,
        is_overdue,
    };

    let response = ApiResponse::success(Some(stats), "Cycle statistics retrieved successfully");
    (StatusCode::OK, Json(response)).into_response()
}

/// 获取周期内的 Issues 列表
pub async fn get_cycle_issues(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
    Query(query): Query<CycleIssuesQuery>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证周期存在且属于当前工作区
    let _cycle_exists = match schema::cycles::table
        .inner_join(schema::teams::table.on(schema::cycles::team_id.eq(schema::teams::id)))
        .filter(schema::cycles::id.eq(cycle_id))
        .filter(schema::teams::workspace_id.eq(current_workspace_id))
        .select(schema::cycles::id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(Some(_)) => true,
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Cycle not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 分页参数
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * limit;

    // 构建基础查询
    let base_query = schema::issues::table
        .left_join(
            schema::workflow_states::table
                .on(schema::issues::workflow_state_id.eq(schema::workflow_states::id.nullable())),
        )
        .filter(schema::issues::cycle_id.eq(cycle_id));

    // 构建计数查询
    let mut count_query = base_query.clone().into_boxed();
    if let Some(status) = &query.status {
        count_query = count_query.filter(schema::workflow_states::category.eq(status));
    }

    // 获取总数
    let total = match count_query.count().get_result::<i64>(&mut conn) {
        Ok(count) => count,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to count issues");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 构建分页查询
    let mut issues_query = base_query.into_boxed();
    if let Some(status) = &query.status {
        issues_query = issues_query.filter(schema::workflow_states::category.eq(status));
    }

    // 获取分页数据
    let issues = match issues_query
        .select(Issue::as_select())
        .order(schema::issues::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load::<Issue>(&mut conn)
    {
        Ok(issues) => issues,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve issues");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let total_pages = (total + limit - 1) / limit;

    let paginated_issues = PaginatedIssues {
        issues,
        total,
        page,
        limit,
        total_pages,
    };

    let response = ApiResponse::success(
        Some(paginated_issues),
        "Cycle issues retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

/// 将 Issues 分配到周期
pub async fn assign_issues_to_cycle(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
    Json(payload): Json<AssignIssuesToCycleRequest>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证周期存在且属于当前工作区
    match schema::cycles::table
        .inner_join(schema::teams::table.on(schema::cycles::team_id.eq(schema::teams::id)))
        .filter(schema::cycles::id.eq(cycle_id))
        .filter(schema::teams::workspace_id.eq(current_workspace_id))
        .select(schema::cycles::id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Cycle not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 批量更新 issues 的 cycle_id
    match diesel::update(
        schema::issues::table.filter(schema::issues::id.eq_any(&payload.issue_ids)),
    )
    .set(schema::issues::cycle_id.eq(Some(cycle_id)))
    .execute(&mut conn)
    {
        Ok(updated_count) => {
            let response = ApiResponse::success(
                Some(format!(
                    "Successfully assigned {} issues to cycle",
                    updated_count
                )),
                "Issues assigned to cycle successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to assign issues to cycle");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 从周期中移除 Issues
pub async fn remove_issues_from_cycle(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(cycle_id): Path<Uuid>,
    Json(payload): Json<AssignIssuesToCycleRequest>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 验证周期存在且属于当前工作区
    match schema::cycles::table
        .inner_join(schema::teams::table.on(schema::cycles::team_id.eq(schema::teams::id)))
        .filter(schema::cycles::id.eq(cycle_id))
        .filter(schema::teams::workspace_id.eq(current_workspace_id))
        .select(schema::cycles::id)
        .first::<Uuid>(&mut conn)
        .optional()
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Cycle not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 批量更新 issues 的 cycle_id 为 NULL
    match diesel::update(
        schema::issues::table
            .filter(schema::issues::id.eq_any(&payload.issue_ids))
            .filter(schema::issues::cycle_id.eq(Some(cycle_id))),
    )
    .set(schema::issues::cycle_id.eq(None::<Uuid>))
    .execute(&mut conn)
    {
        Ok(updated_count) => {
            let response = ApiResponse::success(
                Some(format!(
                    "Successfully removed {} issues from cycle",
                    updated_count
                )),
                "Issues removed from cycle successfully",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to remove issues from cycle");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// 自动更新周期状态
pub async fn update_cycle_status_auto(State(pool): State<Arc<DbPool>>) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let today = chrono::Utc::now().date_naive();

    // 将已过开始日期但状态为 planned 的周期更新为 active
    let activated_count = match diesel::update(
        schema::cycles::table
            .filter(schema::cycles::status.eq("planned"))
            .filter(schema::cycles::start_date.le(today)),
    )
    .set(schema::cycles::status.eq("active"))
    .execute(&mut conn)
    {
        Ok(count) => count,
        Err(_) => 0,
    };

    // 将已过结束日期但状态为 active 的周期更新为 completed
    let completed_count = match diesel::update(
        schema::cycles::table
            .filter(schema::cycles::status.eq("active"))
            .filter(schema::cycles::end_date.lt(today)),
    )
    .set(schema::cycles::status.eq("completed"))
    .execute(&mut conn)
    {
        Ok(count) => count,
        Err(_) => 0,
    };

    let message = format!(
        "Auto-updated {} cycles to active and {} cycles to completed",
        activated_count, completed_count
    );

    let response = ApiResponse::success(Some(message), "Cycle statuses updated automatically");
    (StatusCode::OK, Json(response)).into_response()
}
