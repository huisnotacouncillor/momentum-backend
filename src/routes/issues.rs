use crate::AppState;
use crate::db::models::*;
use crate::db::enums::{IssuePriority, IssueStatus};
use axum::{
    Json, 
    http::StatusCode, 
    response::IntoResponse,
    extract::{Path, Query, State},
};
use std::sync::Arc;
use serde::Deserialize;
use uuid::Uuid;
use crate::middleware::auth::AuthUserInfo;
use diesel::prelude::*;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct IssueQueryParams {
    pub team_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub search: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub description: Option<String>,
    pub project_id: Uuid,
    pub team_id: Option<Uuid>,
    pub priority: Option<IssuePriority>,
    pub status: Option<IssueStatus>,
    pub assignee_id: Option<Uuid>,
    pub reporter_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub priority: Option<IssuePriority>,
    pub status: Option<IssueStatus>,
    pub assignee_id: Option<Uuid>,
}

// 为IssueStatus实现FromStr trait
impl FromStr for IssueStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(IssueStatus::Backlog),
            "todo" => Ok(IssueStatus::Todo),
            "in_progress" => Ok(IssueStatus::InProgress),
            "in_review" => Ok(IssueStatus::InReview),
            "done" => Ok(IssueStatus::Done),
            "canceled" => Ok(IssueStatus::Canceled),
            _ => Err(()),
        }
    }
}

// 为IssuePriority实现FromStr trait
impl FromStr for IssuePriority {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(IssuePriority::None),
            "low" => Ok(IssuePriority::Low),
            "medium" => Ok(IssuePriority::Medium),
            "high" => Ok(IssuePriority::High),
            "urgent" => Ok(IssuePriority::Urgent),
            _ => Err(()),
        }
    }
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

pub async fn create_issue(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateIssueRequest>,
) -> impl IntoResponse {
    let user_id = auth_info.user.id;
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    
    // Begin a transaction for this query
    let transaction_result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // 检查项目是否存在且属于当前工作区
        use crate::schema::projects::dsl::*;
        let project = projects
            .filter(id.eq(payload.project_id))
            .filter(workspace_id.eq(current_workspace_id))
            .select(Project::as_select())
            .first::<Project>(conn)?;

        // 如果指定了团队，验证团队是否存在且属于当前工作区
        if let Some(team_id) = payload.team_id {
            use crate::schema::teams::dsl::*;
            teams
                .filter(id.eq(team_id))
                .filter(workspace_id.eq(current_workspace_id))
                .select(Team::as_select())
                .first::<Team>(conn)?;
        }

        // 如果指定了负责人，验证用户是否是工作区成员
        if let Some(assignee_id) = payload.assignee_id {
            use crate::schema::workspace_members::dsl::*;
            workspace_members
                .filter(user_id.eq(assignee_id))
                .filter(workspace_id.eq(current_workspace_id))
                .select(WorkspaceMember::as_select())
                .first::<WorkspaceMember>(conn)?;
        }

        // 如果指定了报告人，验证用户是否是工作区成员
        let reporter_id = payload.reporter_id.unwrap_or(user_id);
        if reporter_id != user_id {
            use crate::schema::workspace_members::dsl::*;
            workspace_members
                .filter(user_id.eq(reporter_id))
                .filter(workspace_id.eq(current_workspace_id))
                .select(WorkspaceMember::as_select())
                .first::<WorkspaceMember>(conn)?;
        }

        // 创建问题
        let new_issue = NewIssue {
            title: payload.title,
            description: payload.description,
            project_id: Some(payload.project_id),
            cycle_id: None,
            team_id: payload.team_id.unwrap_or(project.id), // 使用project.id作为默认team_id
            priority: Some(match payload.priority.unwrap_or(IssuePriority::None) {
                IssuePriority::None => "none".to_string(),
                IssuePriority::Low => "low".to_string(),
                IssuePriority::Medium => "medium".to_string(),
                IssuePriority::High => "high".to_string(),
                IssuePriority::Urgent => "urgent".to_string(),
            }),
            status: Some(match payload.status.unwrap_or(IssueStatus::Todo) {
                IssueStatus::Backlog => "backlog".to_string(),
                IssueStatus::Todo => "todo".to_string(),
                IssueStatus::InProgress => "in_progress".to_string(),
                IssueStatus::InReview => "in_review".to_string(),
                IssueStatus::Done => "done".to_string(),
                IssueStatus::Canceled => "canceled".to_string(),
            }),
            creator_id: user_id,
            assignee_id: payload.assignee_id,
            parent_issue_id: None,
            is_changelog_candidate: Some(false),
        };

        let issue = diesel::insert_into(crate::schema::issues::table)
            .values(&new_issue)
            .get_result::<Issue>(conn)?;
        
        Ok(issue)
    });

    let issue = match transaction_result {
        Ok(issue) => issue,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to create issue");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(issue),
        "Issue created successfully",
    );
    (StatusCode::CREATED, Json(response)).into_response()
}

pub async fn get_issues(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Query(params): Query<IssueQueryParams>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let mut query = crate::schema::issues::table
        .inner_join(crate::schema::projects::table.on(
            crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())
        ))
        .filter(crate::schema::projects::workspace_id.eq(current_workspace_id))
        .select(Issue::as_select())
        .order(crate::schema::issues::created_at.desc())
        .into_boxed();

    if let Some(team_id_param) = params.team_id {
        query = query.filter(crate::schema::issues::team_id.eq(team_id_param));
    }

    if let Some(project_id_param) = params.project_id {
        query = query.filter(crate::schema::issues::project_id.eq(project_id_param));
    }

    if let Some(assignee_id_param) = params.assignee_id {
        query = query.filter(crate::schema::issues::assignee_id.eq(assignee_id_param));
    }

    if let Some(status_str) = params.status {
        if status_str.parse::<IssueStatus>().is_ok() {
            query = query.filter(crate::schema::issues::status.eq(status_str));
        }
    }

    if let Some(priority_str) = params.priority {
        if priority_str.parse::<IssuePriority>().is_ok() {
            query = query.filter(crate::schema::issues::priority.eq(priority_str));
        }
    }

    if let Some(search_term) = params.search {
        let pattern = format!("%{}%", search_term.to_lowercase());
        query = query.filter(crate::schema::issues::title.ilike(pattern));
    }

    let issues_list = match query.load::<Issue>(&mut conn) {
        Ok(list) => list,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to retrieve issues");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(issues_list),
        "Issues retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn get_issue_by_id(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(issue_id): Path<Uuid>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let issue = match crate::schema::issues::table
        .inner_join(crate::schema::projects::table.on(
            crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())
        ))
        .filter(crate::schema::issues::id.eq(issue_id))
        .filter(crate::schema::projects::workspace_id.eq(current_workspace_id))
        .select(Issue::as_select())
        .first::<Issue>(&mut conn)
    {
        Ok(issue) => issue,
        Err(_) => {
            let response = ApiResponse::<()>::not_found("Issue not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(issue),
        "Issue retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn update_issue(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(issue_id): Path<Uuid>,
    Json(payload): Json<UpdateIssueRequest>,
) -> impl IntoResponse {
    let _user_id = auth_info.user.id;
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查问题是否存在
    let existing_issue = match crate::schema::issues::table
        .inner_join(crate::schema::projects::table.on(
            crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())
        ))
        .filter(crate::schema::issues::id.eq(issue_id))
        .filter(crate::schema::projects::workspace_id.eq(current_workspace_id))
        .select(Issue::as_select())
        .first::<Issue>(&mut conn)
        .optional()
    {
        Ok(Some(issue)) => issue,
        Ok(None) => {
            let response = ApiResponse::<()>::not_found("Issue not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database error");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查用户是否有权限更新问题
    let project_id = existing_issue.project_id;
    let _project = match project_id {
        Some(pid) => {
            match crate::schema::projects::table
                .filter(crate::schema::projects::id.eq(pid))
                .filter(crate::schema::projects::workspace_id.eq(current_workspace_id))
                .select(Project::as_select())
                .first::<Project>(&mut conn)
                .optional()
            {
                Ok(Some(project)) => Some(project),
                Ok(None) => {
                    let response = ApiResponse::<()>::not_found("Project not found");
                    return (StatusCode::NOT_FOUND, Json(response)).into_response();
                }
                Err(_) => {
                    let response = ApiResponse::<()>::internal_error("Database error");
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                }
            }
        }
        None => None,
    };

    // 如果指定了项目，验证项目是否存在且属于当前工作区
    if let Some(project_id_param) = payload.project_id {
        use crate::schema::projects::dsl::*;
        match projects
            .filter(id.eq(project_id_param))
            .filter(workspace_id.eq(current_workspace_id))
            .select(Project::as_select())
            .first::<Project>(&mut conn)
        {
            Ok(_) => (),
            Err(_) => {
                let response = ApiResponse::<()>::not_found("Project not found in current workspace");
                return (StatusCode::NOT_FOUND, Json(response)).into_response();
            }
        }
    }

    // 如果指定了团队，验证团队是否存在且属于当前工作区
    if let Some(team_id_param) = payload.team_id {
        use crate::schema::teams::dsl::*;
        match teams
            .filter(id.eq(team_id_param))
            .filter(workspace_id.eq(current_workspace_id))
            .select(Team::as_select())
            .first::<Team>(&mut conn)
        {
            Ok(_) => (),
            Err(_) => {
                let response = ApiResponse::<()>::not_found("Team not found in current workspace");
                return (StatusCode::NOT_FOUND, Json(response)).into_response();
            }
        }
    }

    // 如果指定了负责人，验证用户是否是工作区成员
    if let Some(assignee_id_param) = payload.assignee_id {
        use crate::schema::workspace_members::dsl::*;
        match workspace_members
            .filter(user_id.eq(assignee_id_param))
            .filter(workspace_id.eq(current_workspace_id))
            .select(WorkspaceMember::as_select())
            .first::<WorkspaceMember>(&mut conn)
        {
            Ok(_) => (),
            Err(_) => {
                let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                    field: Some("assignee_id".to_string()),
                    code: "INVALID".to_string(),
                    message: "Assignee is not a member of the workspace".to_string(),
                }]);
                return (StatusCode::BAD_REQUEST, Json(response)).into_response();
            }
        }
    }

    // 构建更新查询
    let title = match &payload.title {
        Some(title) => {
            if title.trim().is_empty() {
                let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                    field: Some("title".to_string()),
                    code: "REQUIRED".to_string(),
                    message: "Issue title cannot be empty".to_string(),
                }]);
                return (StatusCode::BAD_REQUEST, Json(response)).into_response();
            }
            title.clone()
        }
        None => existing_issue.title,
    };

    let description = match payload.description {
        Some(ref description) => Some(description.clone()),
        None => existing_issue.description,
    };

    let project_id_value = match payload.project_id {
        Some(project_id_param) => Some(project_id_param),
        None => existing_issue.project_id,
    };

    let team_id_value = match payload.team_id {
        Some(team_id_param) => team_id_param,
        None => existing_issue.team_id,
    };

    let priority_value = match payload.priority {
        Some(priority) => Some(priority),
        None => {
            match existing_issue.priority.as_str() {
                "none" => Some(IssuePriority::None),
                "low" => Some(IssuePriority::Low),
                "medium" => Some(IssuePriority::Medium),
                "high" => Some(IssuePriority::High),
                "urgent" => Some(IssuePriority::Urgent),
                _ => None,
            }
        }
    }.unwrap_or(IssuePriority::None);

    let status = match payload.status {
        Some(other_status) => Some(other_status),
        None => {
            match existing_issue.status.as_str() {
                "backlog" => Some(IssueStatus::Backlog),
                "todo" => Some(IssueStatus::Todo),
                "in_progress" => Some(IssueStatus::InProgress),
                "in_review" => Some(IssueStatus::InReview),
                "done" => Some(IssueStatus::Done),
                "canceled" => Some(IssueStatus::Canceled),
                _ => None,
            }
        }
    }.unwrap_or(IssueStatus::Todo);

    let assignee_id_val = match payload.assignee_id {
        Some(assignee_id_param) => Some(assignee_id_param),
        None => existing_issue.assignee_id,
    };

    let update_query = diesel::update(crate::schema::issues::table.filter(crate::schema::issues::id.eq(issue_id)))
        .set((
            crate::schema::issues::title.eq(title),
            crate::schema::issues::description.eq(description),
            crate::schema::issues::project_id.eq(project_id_value),
            crate::schema::issues::team_id.eq(team_id_value),
            crate::schema::issues::priority.eq(match priority_value {
                IssuePriority::None => "none".to_string(),
                IssuePriority::Low => "low".to_string(),
                IssuePriority::Medium => "medium".to_string(),
                IssuePriority::High => "high".to_string(),
                IssuePriority::Urgent => "urgent".to_string(),
            }),
            crate::schema::issues::status.eq(match status {
                IssueStatus::Backlog => "backlog".to_string(),
                IssueStatus::Todo => "todo".to_string(),
                IssueStatus::InProgress => "in_progress".to_string(),
                IssueStatus::InReview => "in_review".to_string(),
                IssueStatus::Done => "done".to_string(),
                IssueStatus::Canceled => "canceled".to_string(),
            }),
            crate::schema::issues::assignee_id.eq(assignee_id_val),
        ));
    
    // 执行更新查询
    let updated_issue = match update_query.get_result::<Issue>(&mut conn) {
        Ok(issue) => issue,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update issue");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::success(
        Some(updated_issue),
        "Issue updated successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

pub async fn delete_issue(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    Path(issue_id): Path<Uuid>,
) -> impl IntoResponse {
    let current_workspace_id = auth_info.current_workspace_id.unwrap();

    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 检查问题是否存在且属于当前工作区
    let issue_exists = match crate::schema::issues::table
        .inner_join(crate::schema::projects::table.on(
            crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())
        ))
        .filter(crate::schema::issues::id.eq(issue_id))
        .filter(crate::schema::projects::workspace_id.eq(current_workspace_id))
        .select(crate::schema::issues::id)
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

    if !issue_exists {
        let response = ApiResponse::<()>::not_found("Issue not found");
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    // 删除问题
    match diesel::delete(crate::schema::issues::table.filter(crate::schema::issues::id.eq(issue_id))).execute(&mut conn) {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Issue deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        },
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to delete issue");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}