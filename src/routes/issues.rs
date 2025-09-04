use crate::AppState;
use crate::db::enums::{IssuePriority, IssueStatus};
use crate::db::models::*;
use crate::middleware::auth::AuthUserInfo;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

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

#[derive(Serialize)]
pub struct IssueListItem {
    pub issue: crate::db::models::issue::IssueResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<crate::db::models::auth::UserBasicInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_issue: Option<crate::db::models::issue::IssueResponse>,
    #[serde(default)]
    pub child_issues: Vec<crate::db::models::issue::IssueResponse>,
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

    let response = ApiResponse::success(Some(issue), "Issue created successfully");
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
        .inner_join(
            crate::schema::projects::table
                .on(crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())),
        )
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

    // Batch load assignees, parent issues, and child issues
    let issue_ids: Vec<Uuid> = issues_list.iter().map(|i| i.id).collect();
    let assignee_ids: Vec<Uuid> = issues_list.iter().filter_map(|i| i.assignee_id).collect();
    let parent_ids: Vec<Uuid> = issues_list
        .iter()
        .filter_map(|i| i.parent_issue_id)
        .collect();

    // Load assignees
    let assignees_map: HashMap<Uuid, crate::db::models::auth::UserBasicInfo> =
        if !assignee_ids.is_empty() {
            use crate::schema::users::dsl as users_dsl;
            match users_dsl::users
                .filter(users_dsl::id.eq_any(&assignee_ids))
                .select((
                    users_dsl::id,
                    users_dsl::name,
                    users_dsl::username,
                    users_dsl::email,
                    users_dsl::avatar_url,
                ))
                .load::<(Uuid, String, String, String, Option<String>)>(&mut conn)
            {
                Ok(rows) => rows
                    .into_iter()
                    .map(|(id, name, username, email, avatar_url)| {
                        (
                            id,
                            crate::db::models::auth::UserBasicInfo {
                                id,
                                name,
                                username,
                                email,
                                avatar_url,
                            },
                        )
                    })
                    .collect(),
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

    // Load parent issues
    let parents_map: HashMap<Uuid, crate::db::models::issue::IssueResponse> =
        if !parent_ids.is_empty() {
            match crate::schema::issues::table
                .filter(crate::schema::issues::id.eq_any(&parent_ids))
                .select(Issue::as_select())
                .load::<Issue>(&mut conn)
            {
                Ok(parents) => parents
                    .into_iter()
                    .map(|iss| {
                        let resp: crate::db::models::issue::IssueResponse = iss.into();
                        (resp.id, resp)
                    })
                    .collect(),
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

    // Load child issues grouped by parent_issue_id
    let mut children_map: HashMap<Uuid, Vec<crate::db::models::issue::IssueResponse>> =
        HashMap::new();
    if !issue_ids.is_empty() {
        match crate::schema::issues::table
            .filter(crate::schema::issues::parent_issue_id.eq_any(&issue_ids))
            .select(Issue::as_select())
            .load::<Issue>(&mut conn)
        {
            Ok(children) => {
                for child in children {
                    let parent_id = child.parent_issue_id.unwrap();
                    let resp: crate::db::models::issue::IssueResponse = child.into();
                    children_map.entry(parent_id).or_default().push(resp);
                }
            }
            Err(_) => {}
        }
    }

    // Build response items
    let items: Vec<crate::db::models::issue::IssueResponse> = issues_list
        .into_iter()
        .map(|iss| {
            let issue_resp: crate::db::models::issue::IssueResponse = iss.clone().into();
            let mut issue_resp = issue_resp;
            issue_resp.assignee = iss
                .assignee_id
                .and_then(|uid| assignees_map.get(&uid).cloned());
            issue_resp.parent_issue = iss
                .parent_issue_id
                .and_then(|pid| parents_map.get(&pid).cloned())
                .map(Box::new);
            issue_resp.child_issues = children_map.remove(&iss.id).unwrap_or_default();
            issue_resp
        })
        .collect();

    let response = ApiResponse::success(items, "Issues retrieved successfully");
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
        .inner_join(
            crate::schema::projects::table
                .on(crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())),
        )
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

    // Load nested fields for single issue
    let mut issue_resp: crate::db::models::issue::IssueResponse = issue.clone().into();

    // assignee
    if let Some(uid) = issue.assignee_id {
        use crate::schema::users::dsl as users_dsl;
        if let Ok((id, name, username, email, avatar_url)) = users_dsl::users
            .filter(users_dsl::id.eq(uid))
            .select((
                users_dsl::id,
                users_dsl::name,
                users_dsl::username,
                users_dsl::email,
                users_dsl::avatar_url,
            ))
            .first::<(Uuid, String, String, String, Option<String>)>(&mut conn)
        {
            issue_resp.assignee = Some(crate::db::models::auth::UserBasicInfo {
                id,
                name,
                username,
                email,
                avatar_url,
            });
        }
    }

    // parent issue
    if let Some(pid) = issue.parent_issue_id {
        if let Ok(parent_issue) = crate::schema::issues::table
            .filter(crate::schema::issues::id.eq(pid))
            .select(Issue::as_select())
            .first::<Issue>(&mut conn)
        {
            let parent_resp: crate::db::models::issue::IssueResponse = parent_issue.into();
            issue_resp.parent_issue = Some(Box::new(parent_resp));
        }
    }

    // child issues
    if let Ok(children) = crate::schema::issues::table
        .filter(crate::schema::issues::parent_issue_id.eq(issue.id))
        .select(Issue::as_select())
        .load::<Issue>(&mut conn)
    {
        issue_resp.child_issues = children.into_iter().map(|c| c.into()).collect();
    }

    let response = ApiResponse::success(issue_resp, "Issue retrieved successfully");
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
        .inner_join(
            crate::schema::projects::table
                .on(crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())),
        )
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
                let response =
                    ApiResponse::<()>::not_found("Project not found in current workspace");
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
        None => match existing_issue.priority.as_str() {
            "none" => Some(IssuePriority::None),
            "low" => Some(IssuePriority::Low),
            "medium" => Some(IssuePriority::Medium),
            "high" => Some(IssuePriority::High),
            "urgent" => Some(IssuePriority::Urgent),
            _ => None,
        },
    }
    .unwrap_or(IssuePriority::None);

    let status = match payload.status {
        Some(other_status) => Some(other_status),
        None => match existing_issue.status.as_str() {
            "backlog" => Some(IssueStatus::Backlog),
            "todo" => Some(IssueStatus::Todo),
            "in_progress" => Some(IssueStatus::InProgress),
            "in_review" => Some(IssueStatus::InReview),
            "done" => Some(IssueStatus::Done),
            "canceled" => Some(IssueStatus::Canceled),
            _ => None,
        },
    }
    .unwrap_or(IssueStatus::Todo);

    let assignee_id_val = match payload.assignee_id {
        Some(assignee_id_param) => Some(assignee_id_param),
        None => existing_issue.assignee_id,
    };

    let update_query =
        diesel::update(crate::schema::issues::table.filter(crate::schema::issues::id.eq(issue_id)))
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

    let response = ApiResponse::success(Some(updated_issue), "Issue updated successfully");
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
        .inner_join(
            crate::schema::projects::table
                .on(crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())),
        )
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
    match diesel::delete(
        crate::schema::issues::table.filter(crate::schema::issues::id.eq(issue_id)),
    )
    .execute(&mut conn)
    {
        Ok(_) => {
            let response = ApiResponse::<()>::success((), "Issue deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to delete issue");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}
