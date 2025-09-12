use crate::AppState;
use crate::db::enums::IssuePriority;
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
    pub priority: Option<String>,
    pub search: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Uuid,
    pub priority: Option<IssuePriority>,
    pub assignee_id: Option<Uuid>,
    pub reporter_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
    pub cycle_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub priority: Option<IssuePriority>,
    pub assignee_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
    pub cycle_id: Option<Uuid>,
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
        // 如果指定了项目，检查项目是否存在且属于当前工作区
        let _project = if let Some(project_id) = payload.project_id {
            use crate::schema::projects::dsl::*;
            Some(
                projects
                    .filter(id.eq(project_id))
                    .filter(workspace_id.eq(current_workspace_id))
                    .select(Project::as_select())
                    .first::<Project>(conn)?,
            )
        } else {
            None
        };

        // 验证团队是否存在且属于当前工作区
        use crate::schema::teams::dsl::*;
        teams
            .filter(id.eq(payload.team_id))
            .filter(workspace_id.eq(current_workspace_id))
            .select(Team::as_select())
            .first::<Team>(conn)?;

        // 如果指定了负责人，验证用户是否是工作区成员
        if let Some(assignee_id) = payload.assignee_id {
            use crate::schema::workspace_members::dsl::*;
            workspace_members
                .filter(user_id.eq(assignee_id))
                .filter(workspace_id.eq(current_workspace_id))
                .select(WorkspaceMember::as_select())
                .first::<WorkspaceMember>(conn)?;
        }

        // 如果指定了父任务，验证其属于当前工作区
        if let Some(parent_id) = payload.parent_issue_id {
            // 加载父任务
            let parent_issue = crate::schema::issues::table
                .filter(crate::schema::issues::id.eq(parent_id))
                .select(Issue::as_select())
                .first::<Issue>(conn)?;

            // 确认父任务所在团队属于当前工作区
            use crate::schema::teams::dsl as teams_dsl2;
            teams_dsl2::teams
                .filter(teams_dsl2::id.eq(parent_issue.team_id))
                .filter(teams_dsl2::workspace_id.eq(current_workspace_id))
                .select(Team::as_select())
                .first::<Team>(conn)?;
        }

        // 如果只传了 workflow_state_id，需要倒查 workflow_id
        let (final_workflow_id, final_workflow_state_id) =
            match (payload.workflow_id, payload.workflow_state_id) {
                (Some(wf_id), Some(wf_state_id)) => (Some(wf_id), Some(wf_state_id)),
                (None, Some(wf_state_id)) => {
                    // 根据 workflow_state_id 查询 workflow_id
                    use crate::schema::workflow_states::dsl::*;
                    match workflow_states
                        .filter(id.eq(wf_state_id))
                        .select(workflow_id)
                        .first::<uuid::Uuid>(conn)
                    {
                        Ok(wf_id) => (Some(wf_id), Some(wf_state_id)),
                        Err(_) => return Err(diesel::result::Error::NotFound), // workflow_state_id 不存在
                    }
                }
                (Some(wf_id), None) => (Some(wf_id), None),
                (None, None) => (None, None),
            };

        // 创建问题
        let new_issue = NewIssue {
            title: payload.title,
            description: payload.description,
            project_id: payload.project_id,
            cycle_id: payload.cycle_id,
            team_id: payload.team_id,
            priority: Some(match payload.priority.unwrap_or(IssuePriority::None) {
                IssuePriority::None => "none".to_string(),
                IssuePriority::Low => "low".to_string(),
                IssuePriority::Medium => "medium".to_string(),
                IssuePriority::High => "high".to_string(),
                IssuePriority::Urgent => "urgent".to_string(),
            }),
            creator_id: user_id,
            assignee_id: payload.assignee_id,
            parent_issue_id: payload.parent_issue_id,
            is_changelog_candidate: Some(false),
            workflow_id: final_workflow_id,
            workflow_state_id: final_workflow_state_id,
        };

        let issue = diesel::insert_into(crate::schema::issues::table)
            .values(&new_issue)
            .returning(Issue::as_returning())
            .get_result(conn)?;

        // Handle label associations if provided
        if let Some(label_ids) = payload.label_ids {
            if !label_ids.is_empty() {
                // Validate that all labels exist and belong to the current workspace
                use crate::schema::labels::dsl as labels_dsl;
                let valid_labels: Vec<Uuid> = labels_dsl::labels
                    .filter(labels_dsl::id.eq_any(&label_ids))
                    .filter(labels_dsl::workspace_id.eq(current_workspace_id))
                    .select(labels_dsl::id)
                    .load::<Uuid>(conn)?;

                if valid_labels.len() != label_ids.len() {
                    return Err(diesel::result::Error::NotFound);
                }

                // Insert issue-label associations
                let issue_labels: Vec<NewIssueLabel> = valid_labels
                    .into_iter()
                    .map(|label_id| NewIssueLabel {
                        issue_id: issue.id,
                        label_id,
                    })
                    .collect();

                diesel::insert_into(crate::schema::issue_labels::table)
                    .values(&issue_labels)
                    .execute(conn)?;
            }
        }

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
        .left_join(
            crate::schema::projects::table
                .on(crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())),
        )
        .filter(
            crate::schema::projects::workspace_id
                .eq(current_workspace_id)
                .or(crate::schema::issues::project_id.is_null()),
        )
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

    // Batch load assignees, parent issues, child issues, workflow states, and teams
    let issue_ids: Vec<Uuid> = issues_list.iter().map(|i| i.id).collect();
    let assignee_ids: Vec<Uuid> = issues_list.iter().filter_map(|i| i.assignee_id).collect();
    let parent_ids: Vec<Uuid> = issues_list
        .iter()
        .filter_map(|i| i.parent_issue_id)
        .collect();
    let workflow_ids: Vec<Uuid> = issues_list.iter().filter_map(|i| i.workflow_id).collect();
    let team_ids: Vec<Uuid> = issues_list.iter().map(|i| i.team_id).collect();
    let _project_ids: Vec<Uuid> = issues_list.iter().filter_map(|i| i.project_id).collect();
    let _cycle_ids: Vec<Uuid> = issues_list.iter().filter_map(|i| i.cycle_id).collect();

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

    // Load workflow states grouped by workflow_id
    let mut workflow_states_map: HashMap<
        Uuid,
        Vec<crate::db::models::workflow::WorkflowStateResponse>,
    > = HashMap::new();
    if !workflow_ids.is_empty() {
        match crate::schema::workflow_states::table
            .filter(crate::schema::workflow_states::workflow_id.eq_any(&workflow_ids))
            .order(crate::schema::workflow_states::position.asc())
            .select(crate::db::models::workflow::WorkflowState::as_select())
            .load::<crate::db::models::workflow::WorkflowState>(&mut conn)
        {
            Ok(states) => {
                for state in states {
                    let workflow_id = state.workflow_id;
                    let resp: crate::db::models::workflow::WorkflowStateResponse = state.into();
                    workflow_states_map
                        .entry(workflow_id)
                        .or_default()
                        .push(resp);
                }
            }
            Err(_) => {}
        }
    }

    // Load teams
    let teams_map: HashMap<Uuid, String> = if !team_ids.is_empty() {
        use crate::schema::teams::dsl as teams_dsl;
        match teams_dsl::teams
            .filter(teams_dsl::id.eq_any(&team_ids))
            .select((teams_dsl::id, teams_dsl::team_key))
            .load::<(Uuid, String)>(&mut conn)
        {
            Ok(rows) => rows.into_iter().collect(),
            Err(_) => HashMap::new(),
        }
    } else {
        HashMap::new()
    };

    // Load full team information
    let teams_info_map: HashMap<Uuid, crate::db::models::team::TeamBasicInfo> =
        if !team_ids.is_empty() {
            use crate::schema::teams::dsl as teams_dsl;
            match teams_dsl::teams
                .filter(teams_dsl::id.eq_any(&team_ids))
                .select((
                    teams_dsl::id,
                    teams_dsl::name,
                    teams_dsl::team_key,
                    teams_dsl::description,
                    teams_dsl::icon_url,
                    teams_dsl::is_private,
                ))
                .load::<(Uuid, String, String, Option<String>, Option<String>, bool)>(&mut conn)
            {
                Ok(rows) => rows
                    .into_iter()
                    .map(|(id, name, team_key, description, icon_url, is_private)| {
                        (
                            id,
                            crate::db::models::team::TeamBasicInfo {
                                id,
                                name,
                                team_key,
                                description,
                                icon_url,
                                is_private,
                            },
                        )
                    })
                    .collect(),
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

    // Load labels grouped by issue_id
    let mut labels_map: HashMap<Uuid, Vec<crate::db::models::label::Label>> = HashMap::new();
    if !issue_ids.is_empty() {
        match crate::schema::issue_labels::table
            .inner_join(
                crate::schema::labels::table
                    .on(crate::schema::issue_labels::label_id.eq(crate::schema::labels::id)),
            )
            .filter(crate::schema::issue_labels::issue_id.eq_any(&issue_ids))
            .select((
                crate::schema::issue_labels::issue_id,
                crate::db::models::label::Label::as_select(),
            ))
            .load::<(Uuid, crate::db::models::label::Label)>(&mut conn)
        {
            Ok(label_pairs) => {
                for (issue_id, label) in label_pairs {
                    labels_map.entry(issue_id).or_default().push(label);
                }
            }
            Err(_) => {}
        }
    }

    // Collect project_ids and cycle_ids
    let project_ids: Vec<Uuid> = issues_list
        .iter()
        .filter_map(|iss| iss.project_id)
        .collect();
    let cycle_ids: Vec<Uuid> = issues_list.iter().filter_map(|iss| iss.cycle_id).collect();

    // Load projects basic info
    let projects_map: HashMap<Uuid, crate::db::models::project::ProjectInfo> = if !project_ids
        .is_empty()
    {
        match crate::schema::projects::table
            .filter(crate::schema::projects::id.eq_any(&project_ids))
            .select((
                crate::schema::projects::id,
                crate::schema::projects::name,
                crate::schema::projects::project_key,
                crate::schema::projects::description,
                crate::schema::projects::target_date,
                crate::schema::projects::created_at,
                crate::schema::projects::updated_at,
            ))
            .load::<(
                Uuid,
                String,
                String,
                Option<String>,
                Option<chrono::NaiveDate>,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
            )>(&mut conn)
        {
            Ok(projects) => projects
                .into_iter()
                .map(
                    |(id, name, project_key, description, target_date, created_at, updated_at)| {
                        let project_info = crate::db::models::project::ProjectInfo {
                            id,
                            name,
                            project_key,
                            description,
                            status: crate::db::models::project_status::ProjectStatusInfo {
                                id: Uuid::nil(),
                                name: "Unknown".to_string(),
                                description: None,
                                color: Some("#cccccc".to_string()),
                                category: crate::db::models::project_status::ProjectStatusCategory::Backlog,
                                created_at: chrono::Utc::now(),
                                updated_at: chrono::Utc::now(),
                            },
                            owner: crate::db::models::auth::UserBasicInfo {
                                id: Uuid::nil(),
                                name: "Unknown".to_string(),
                                username: "unknown".to_string(),
                                email: "unknown@example.com".to_string(),
                                avatar_url: None,
                            },
                            target_date,
                            priority: crate::db::enums::ProjectPriority::None,
                            created_at,
                            updated_at,
                        };
                        (id, project_info)
                    },
                )
                .collect(),
            Err(_) => HashMap::new(),
        }
    } else {
        HashMap::new()
    };

    // Load cycles
    let cycles_map: HashMap<Uuid, crate::db::models::cycle::Cycle> = if !cycle_ids.is_empty() {
        match crate::schema::cycles::table
            .filter(crate::schema::cycles::id.eq_any(&cycle_ids))
            .select(crate::db::models::cycle::Cycle::as_select())
            .load::<crate::db::models::cycle::Cycle>(&mut conn)
        {
            Ok(cycles) => cycles.into_iter().map(|c| (c.id, c)).collect(),
            Err(_) => HashMap::new(),
        }
    } else {
        HashMap::new()
    };

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
            issue_resp.workflow_states = iss
                .workflow_id
                .and_then(|wid| workflow_states_map.get(&wid).cloned())
                .unwrap_or_default();
            issue_resp.team_key = teams_map.get(&iss.team_id).cloned();
            issue_resp.team = teams_info_map.get(&iss.team_id).cloned();
            issue_resp.labels = labels_map.remove(&iss.id).unwrap_or_default();
            issue_resp.project = iss
                .project_id
                .and_then(|pid| projects_map.get(&pid).cloned());
            issue_resp.cycle = iss.cycle_id.and_then(|cid| cycles_map.get(&cid).cloned());
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
        .left_join(
            crate::schema::projects::table
                .on(crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())),
        )
        .filter(crate::schema::issues::id.eq(issue_id))
        .filter(
            crate::schema::projects::workspace_id
                .eq(current_workspace_id)
                .or(crate::schema::issues::project_id.is_null()),
        )
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

    // workflow states
    if let Some(workflow_id) = issue.workflow_id {
        if let Ok(states) = crate::schema::workflow_states::table
            .filter(crate::schema::workflow_states::workflow_id.eq(workflow_id))
            .order(crate::schema::workflow_states::position.asc())
            .select(crate::db::models::workflow::WorkflowState::as_select())
            .load::<crate::db::models::workflow::WorkflowState>(&mut conn)
        {
            issue_resp.workflow_states = states.into_iter().map(|s| s.into()).collect();
        }
    }

    // team_key and team info
    use crate::schema::teams::dsl as teams_dsl;
    if let Ok((_, name, team_key, description, icon_url, is_private)) = teams_dsl::teams
        .filter(teams_dsl::id.eq(issue.team_id))
        .select((
            teams_dsl::id,
            teams_dsl::name,
            teams_dsl::team_key,
            teams_dsl::description,
            teams_dsl::icon_url,
            teams_dsl::is_private,
        ))
        .first::<(Uuid, String, String, Option<String>, Option<String>, bool)>(&mut conn)
    {
        issue_resp.team_key = Some(team_key.clone());
        issue_resp.team = Some(crate::db::models::team::TeamBasicInfo {
            id: issue.team_id,
            name,
            team_key,
            description,
            icon_url,
            is_private,
        });
    }

    // labels
    if let Ok(labels) = crate::schema::issue_labels::table
        .inner_join(
            crate::schema::labels::table
                .on(crate::schema::issue_labels::label_id.eq(crate::schema::labels::id)),
        )
        .filter(crate::schema::issue_labels::issue_id.eq(issue.id))
        .select(crate::db::models::label::Label::as_select())
        .load::<crate::db::models::label::Label>(&mut conn)
    {
        issue_resp.labels = labels;
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
        .left_join(
            crate::schema::projects::table
                .on(crate::schema::issues::project_id.eq(crate::schema::projects::id.nullable())),
        )
        .filter(crate::schema::issues::id.eq(issue_id))
        .filter(
            crate::schema::projects::workspace_id
                .eq(current_workspace_id)
                .or(crate::schema::issues::project_id.is_null()),
        )
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

    // 如果指定了项目，验证项目是否存在且属于当前工作区（跳过空 UUID 的验证）
    if let Some(project_id_param) = payload.project_id {
        if project_id_param != Uuid::nil() {
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

    // 如果指定了负责人，验证用户是否是工作区成员（跳过空 UUID 的验证）
    if let Some(assignee_id_param) = payload.assignee_id {
        if assignee_id_param != Uuid::nil() {
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
    }

    // 如果指定了周期，验证周期是否存在且属于当前工作区（跳过空 UUID 的验证）
    if let Some(cycle_id_param) = payload.cycle_id {
        if cycle_id_param != Uuid::nil() {
            use crate::schema::cycles::dsl as cycles_dsl;
            use crate::schema::teams::dsl as teams_dsl;

            match cycles_dsl::cycles
                .inner_join(teams_dsl::teams.on(cycles_dsl::team_id.eq(teams_dsl::id)))
                .filter(cycles_dsl::id.eq(cycle_id_param))
                .filter(teams_dsl::workspace_id.eq(current_workspace_id))
                .select(Cycle::as_select())
                .first::<Cycle>(&mut conn)
            {
                Ok(_) => (),
                Err(_) => {
                    let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                        field: Some("cycle_id".to_string()),
                        code: "INVALID".to_string(),
                        message: "Cycle not found in current workspace".to_string(),
                    }]);
                    return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                }
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
        Some(project_id_param) => {
            // 支持设置为空：如果传入的是全零的 UUID，则设置为 None
            if project_id_param == Uuid::nil() {
                None
            } else {
                Some(project_id_param)
            }
        }
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

    let assignee_id_val = match payload.assignee_id {
        Some(assignee_id_param) => {
            // 支持设置为空：如果传入的是全零的 UUID，则设置为 None
            if assignee_id_param == Uuid::nil() {
                None
            } else {
                Some(assignee_id_param)
            }
        }
        None => existing_issue.assignee_id,
    };

    let workflow_id_val = match payload.workflow_id {
        Some(workflow_id_param) => Some(workflow_id_param),
        None => existing_issue.workflow_id,
    };

    let workflow_state_id_val = match payload.workflow_state_id {
        Some(workflow_state_id_param) => Some(workflow_state_id_param),
        None => existing_issue.workflow_state_id,
    };

    let cycle_id_val = match payload.cycle_id {
        Some(cycle_id_param) => {
            // 支持设置为空：如果传入的是全零的 UUID，则设置为 None
            if cycle_id_param == Uuid::nil() {
                None
            } else {
                Some(cycle_id_param)
            }
        }
        None => existing_issue.cycle_id,
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
                crate::schema::issues::assignee_id.eq(assignee_id_val),
                crate::schema::issues::workflow_id.eq(workflow_id_val),
                crate::schema::issues::workflow_state_id.eq(workflow_state_id_val),
                crate::schema::issues::cycle_id.eq(cycle_id_val),
            ));

    // 执行更新查询
    let updated_issue = match update_query
        .returning(Issue::as_returning())
        .get_result(&mut conn)
    {
        Ok(issue) => issue,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Failed to update issue");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // Handle label updates if provided
    if let Some(label_ids) = payload.label_ids {
        // Delete existing label associations
        if let Err(_) = diesel::delete(
            crate::schema::issue_labels::table
                .filter(crate::schema::issue_labels::issue_id.eq(issue_id)),
        )
        .execute(&mut conn)
        {
            let response = ApiResponse::<()>::internal_error("Failed to remove existing labels");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }

        // Add new label associations if any
        if !label_ids.is_empty() {
            // Validate that all labels exist and belong to the current workspace
            use crate::schema::labels::dsl as labels_dsl;
            let valid_labels: Vec<Uuid> = match labels_dsl::labels
                .filter(labels_dsl::id.eq_any(&label_ids))
                .filter(labels_dsl::workspace_id.eq(current_workspace_id))
                .select(labels_dsl::id)
                .load::<Uuid>(&mut conn)
            {
                Ok(labels) => labels,
                Err(_) => {
                    let response = ApiResponse::<()>::internal_error("Failed to validate labels");
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                }
            };

            if valid_labels.len() != label_ids.len() {
                let response = ApiResponse::<()>::validation_error(vec![ErrorDetail {
                    field: Some("label_ids".to_string()),
                    code: "INVALID".to_string(),
                    message: "One or more labels not found or not accessible".to_string(),
                }]);
                return (StatusCode::BAD_REQUEST, Json(response)).into_response();
            }

            // Insert new issue-label associations
            let issue_labels: Vec<NewIssueLabel> = valid_labels
                .into_iter()
                .map(|label_id| NewIssueLabel { issue_id, label_id })
                .collect();

            if let Err(_) = diesel::insert_into(crate::schema::issue_labels::table)
                .values(&issue_labels)
                .execute(&mut conn)
            {
                let response = ApiResponse::<()>::internal_error("Failed to add labels");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        }
    }

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
