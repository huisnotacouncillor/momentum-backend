use crate::AppState;
use crate::db::models::*;
use crate::db::enums::{IssuePriority, IssueStatus};
use axum::{
    Json, 
    http::StatusCode, 
    response::IntoResponse,
    extract::{Path, Query, State, TypedHeader},
};
use std::sync::Arc;
use headers::{Authorization, authorization::Bearer};
use serde::Deserialize;
use uuid::Uuid;
use crate::middleware::auth::{AuthService, AuthConfig};
use crate::db::models::auth::AuthUser;
use diesel::prelude::*;

#[derive(Deserialize)]
pub struct IssueQueryParams {
    pub team_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub search: Option<String>,
}

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

async fn get_user_from_token(bearer: &str, state: &Arc<AppState>) -> Result<AuthUser, (StatusCode, Json<ApiResponse<()>>)> {
    // 验证 access_token
    let auth_service = AuthService::new(AuthConfig::default());
    let claims = match auth_service.verify_token(bearer) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return Err((StatusCode::UNAUTHORIZED, Json(response)));
        }
    };

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(response)));
        }
    };

    // 获取用户信息
    use crate::schema::users::dsl::*;
    let user = match users
        .filter(id.eq(claims.sub))
        .filter(is_active.eq(true))
        .select(crate::db::models::User::as_select())
        .first(&mut conn)
    {
        Ok(user) => AuthUser {
            id: user.id,
            email: user.email,
            username: user.username,
            name: user.name,
            avatar_url: user.avatar_url,
        },
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("User not found or inactive");
            return Err((StatusCode::UNAUTHORIZED, Json(response)));
        }
    };

    Ok(user)
}

pub async fn create_issue(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateIssueRequest>,
) -> impl IntoResponse {
    // 验证 access_token 并获取用户信息
    let user = match get_user_from_token(bearer.token(), &state).await {
        Ok(user) => user,
        Err(response) => return response.into_response(),
    };

    use crate::schema::issues;
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to get database connection");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    };

    let new_issue = NewIssue {
        project_id: payload.project_id,
        cycle_id: payload.cycle_id,
        creator_id: user.id,
        assignee_id: payload.assignee_id,
        parent_issue_id: payload.parent_issue_id,
        title: payload.title,
        description: payload.description,
        status: Some("todo".to_string()), // Default status
        priority: Some("none".to_string()), // Default priority
        is_changelog_candidate: Some(payload.is_changelog_candidate),
        team_id: payload.team_id,
    };

    let result = conn.transaction::<Issue, diesel::result::Error, _>(|conn| {
        // 插入issue
        let issue: Issue = diesel::insert_into(issues::table)
            .values(&new_issue)
            .get_result(conn)?;

        Ok(issue)
    });

    match result {
        Ok(issue) => {
            let response = ApiResponse::created(
                IssueResponse::from(issue),
                "Issue created successfully"
            );
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to create issue");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
        }
    }
}

pub async fn get_issues(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Query(params): Query<IssueQueryParams>,
) -> impl IntoResponse {
    // 验证 access_token 并获取用户信息
    let _user = match get_user_from_token(bearer.token(), &state).await {
        Ok(user) => user,
        Err(response) => return response.into_response(),
    };

    use crate::schema::issues;
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to get database connection");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    };

    let result = conn.transaction::<Vec<Issue>, diesel::result::Error, _>(|conn| {
        let mut query = issues::table.into_boxed();

        if let Some(team_id) = params.team_id {
            query = query.filter(issues::team_id.eq(team_id));
        }

        if let Some(project_id) = params.project_id {
            query = query.filter(issues::project_id.eq(project_id));
        }

        if let Some(assignee_id) = params.assignee_id {
            query = query.filter(issues::assignee_id.eq(assignee_id));
        }

        if let Some(status) = params.status {
            query = query.filter(issues::status.eq(status));
        }

        if let Some(priority) = params.priority {
            query = query.filter(issues::priority.eq(priority));
        }

        if let Some(search) = params.search {
            query = query.filter(issues::title.like(format!("%{}%", search)));
        }

        let issues = query
            .order(issues::created_at.desc())
            .load::<Issue>(conn)?;

        Ok(issues)
    });

    match result {
        Ok(issues) => {
            let issue_responses: Vec<IssueResponse> = issues.into_iter().map(IssueResponse::from).collect();

            let meta = ResponseMeta {
                request_id: None,
                pagination: None,
                total_count: Some(issue_responses.len() as i64),
                execution_time_ms: None,
            };

            let response = ApiResponse::success_with_meta(
                issue_responses,
                "Issues retrieved successfully",
                meta
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to retrieve issues");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
        }
    }
}

pub async fn get_issue_by_id(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(issue_id): Path<Uuid>,
) -> impl IntoResponse {
    // 验证 access_token 并获取用户信息
    let _user = match get_user_from_token(bearer.token(), &state).await {
        Ok(user) => user,
        Err(response) => return response.into_response(),
    };

    use crate::schema::issues;
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to get database connection");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    };

    let result = conn.transaction::<Issue, diesel::result::Error, _>(|conn| {
        let issue = issues::table
            .filter(issues::id.eq(issue_id))
            .first::<Issue>(conn)?;

        Ok(issue)
    });

    match result {
        Ok(issue) => {
            let response = ApiResponse::success(
                IssueResponse::from(issue),
                "Issue retrieved successfully"
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(diesel::result::Error::NotFound) => {
            let error_response = ApiResponse::<()>::not_found("Issue not found");
            (StatusCode::NOT_FOUND, Json(error_response)).into_response()
        }
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to retrieve issue");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
        }
    }
}

pub async fn update_issue(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(issue_id): Path<Uuid>,
    Json(payload): Json<UpdateIssueRequest>,
) -> impl IntoResponse {
    // 验证 access_token 并获取用户信息
    let _user = match get_user_from_token(bearer.token(), &state).await {
        Ok(user) => user,
        Err(response) => return response.into_response(),
    };

    use crate::schema::issues;
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to get database connection");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    };

    // 构建更新结构体
    let mut update_data = UpdateIssue::default();
    
    if let Some(team_id) = payload.team_id {
        update_data.team_id = Some(team_id);
    }
    
    if let Some(project_id) = payload.project_id {
        update_data.project_id = Some(Some(project_id));
    }
    
    if let Some(cycle_id) = payload.cycle_id {
        update_data.cycle_id = Some(Some(cycle_id));
    }
    
    if let Some(assignee_id) = payload.assignee_id {
        update_data.assignee_id = Some(Some(assignee_id));
    }
    
    if let Some(parent_issue_id) = payload.parent_issue_id {
        update_data.parent_issue_id = Some(Some(parent_issue_id));
    }
    
    if let Some(title) = payload.title {
        update_data.title = Some(title);
    }
    
    if let Some(description) = payload.description {
        update_data.description = Some(Some(description));
    }
    
    if let Some(status) = payload.status {
        update_data.status = Some(match status {
            IssueStatus::Backlog => "backlog".to_string(),
            IssueStatus::Todo => "todo".to_string(),
            IssueStatus::InProgress => "in_progress".to_string(),
            IssueStatus::InReview => "in_review".to_string(),
            IssueStatus::Done => "done".to_string(),
            IssueStatus::Canceled => "canceled".to_string(),
        });
    }
    
    if let Some(priority) = payload.priority {
        update_data.priority = Some(match priority {
            IssuePriority::None => "none".to_string(),
            IssuePriority::Low => "low".to_string(),
            IssuePriority::Medium => "medium".to_string(),
            IssuePriority::High => "high".to_string(),
            IssuePriority::Urgent => "urgent".to_string(),
        });
    }
    
    if let Some(is_changelog_candidate) = payload.is_changelog_candidate {
        update_data.is_changelog_candidate = Some(is_changelog_candidate);
    }

    let result = conn.transaction::<Issue, diesel::result::Error, _>(|conn| {
        let issue = diesel::update(issues::table.filter(issues::id.eq(issue_id)))
            .set(&update_data)
            .get_result::<Issue>(conn)?;

        Ok(issue)
    });

    match result {
        Ok(issue) => {
            let response = ApiResponse::success(
                IssueResponse::from(issue),
                "Issue updated successfully"
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(diesel::result::Error::NotFound) => {
            let error_response = ApiResponse::<()>::not_found("Issue not found");
            (StatusCode::NOT_FOUND, Json(error_response)).into_response()
        }
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to update issue");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
        }
    }
}

pub async fn delete_issue(
    State(state): State<Arc<AppState>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    Path(issue_id): Path<Uuid>,
) -> impl IntoResponse {
    // 验证 access_token 并获取用户信息
    let _user = match get_user_from_token(bearer.token(), &state).await {
        Ok(user) => user,
        Err(response) => return response.into_response(),
    };

    use crate::schema::issues;
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to get database connection");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    };

    let result = conn.transaction::<usize, diesel::result::Error, _>(|conn| {
        let deleted_rows = diesel::delete(issues::table.filter(issues::id.eq(issue_id)))
            .execute(conn)?;

        Ok(deleted_rows)
    });

    match result {
        Ok(0) => {
            let error_response = ApiResponse::<()>::not_found("Issue not found");
            (StatusCode::NOT_FOUND, Json(error_response)).into_response()
        }
        Ok(_) => {
            let response = ApiResponse::<()>::ok("Issue deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_e) => {
            let error_response = ApiResponse::<()>::internal_error("Failed to delete issue");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
        }
    }
}