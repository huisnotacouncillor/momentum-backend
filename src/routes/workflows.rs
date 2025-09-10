use crate::db::models::ApiResponse;
use crate::db::{DbPool, models::workflow::*};
use crate::middleware::auth::AuthUserInfo;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

/// Get all workflows for a team
pub async fn get_workflows(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    // Verify user has access to the team
    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Check if team belongs to current workspace
    match crate::schema::teams::table
        .filter(crate::schema::teams::id.eq(team_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .first::<crate::db::models::team::Team>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Team not found or access denied"
                })),
            )
                .into_response();
        }
    }

    match crate::schema::workflows::table
        .filter(crate::schema::workflows::team_id.eq(team_id))
        .load::<Workflow>(&mut conn)
    {
        Ok(workflows) => {
            let responses: Vec<WorkflowResponse> =
                workflows.into_iter().map(|w| w.into()).collect();
            let response = ApiResponse::success(responses, "Workflows retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to load workflows"
            })),
        )
            .into_response(),
    }
}

/// Get a specific workflow by ID with its states
pub async fn get_workflow_by_id(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Get workflow with team info to verify access
    match crate::schema::workflows::table
        .inner_join(crate::schema::teams::table)
        .filter(crate::schema::workflows::id.eq(workflow_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .select(Workflow::as_select())
        .first::<Workflow>(&mut conn)
    {
        Ok(workflow) => {
            // Get workflow states
            let states = match crate::schema::workflow_states::table
                .filter(crate::schema::workflow_states::workflow_id.eq(workflow_id))
                .order(crate::schema::workflow_states::category.asc())
                .then_order_by(crate::schema::workflow_states::position.asc())
                .load::<WorkflowState>(&mut conn)
            {
                Ok(states) => states.into_iter().map(|s| s.into()).collect(),
                Err(_) => Vec::new(),
            };

            let mut response: WorkflowResponse = workflow.into();
            response.states = states;

            let api_response = ApiResponse::success(response, "Workflow retrieved successfully");
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(diesel::result::Error::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Workflow not found"
            })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to get workflow"
            })),
        )
            .into_response(),
    }
}

/// Create a new workflow
pub async fn create_workflow(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
    Json(request): Json<CreateWorkflowRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Verify team belongs to workspace
    match crate::schema::teams::table
        .filter(crate::schema::teams::id.eq(team_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .first::<crate::db::models::team::Team>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Team not found or access denied"
                })),
            )
                .into_response();
        }
    }

    let is_default = request.is_default.unwrap_or(false);

    // If this is going to be the default workflow, unset other defaults
    if is_default {
        diesel::update(
            crate::schema::workflows::table.filter(crate::schema::workflows::team_id.eq(team_id)),
        )
        .set(crate::schema::workflows::is_default.eq(false))
        .execute(&mut conn)
        .ok();
    }

    let new_workflow = NewWorkflow {
        name: request.name,
        description: request.description,
        team_id,
        is_default,
    };

    match diesel::insert_into(crate::schema::workflows::table)
        .values(&new_workflow)
        .get_result::<Workflow>(&mut conn)
    {
        Ok(workflow) => {
            let response: WorkflowResponse = workflow.into();
            let api_response = ApiResponse::success(response, "Workflow created successfully");
            (StatusCode::CREATED, Json(api_response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to create workflow"
            })),
        )
            .into_response(),
    }
}

/// Update a workflow
pub async fn update_workflow(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
    Json(request): Json<UpdateWorkflowRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Verify workflow belongs to current workspace
    match crate::schema::workflows::table
        .inner_join(crate::schema::teams::table)
        .filter(crate::schema::workflows::id.eq(workflow_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .select(Workflow::as_select())
        .first::<Workflow>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Workflow not found or access denied"
                })),
            )
                .into_response();
        }
    }

    let mut update_data = UpdateWorkflow::default();
    if let Some(name) = request.name {
        update_data.name = Some(name);
    }
    if let Some(description) = request.description {
        update_data.description = Some(Some(description));
    }
    if let Some(is_default) = request.is_default {
        update_data.is_default = Some(is_default);

        // If setting as default, unset other defaults for the same team
        if is_default {
            let team_id = crate::schema::workflows::table
                .filter(crate::schema::workflows::id.eq(workflow_id))
                .select(crate::schema::workflows::team_id)
                .first::<Uuid>(&mut conn)
                .unwrap_or_default();

            diesel::update(
                crate::schema::workflows::table
                    .filter(crate::schema::workflows::team_id.eq(team_id)),
            )
            .set(crate::schema::workflows::is_default.eq(false))
            .execute(&mut conn)
            .ok();
        }
    }

    match diesel::update(
        crate::schema::workflows::table.filter(crate::schema::workflows::id.eq(workflow_id)),
    )
    .set(update_data)
    .get_result::<Workflow>(&mut conn)
    {
        Ok(workflow) => {
            let response: WorkflowResponse = workflow.into();
            let api_response = ApiResponse::success(response, "Workflow updated successfully");
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(diesel::result::Error::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Workflow not found"
            })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to update workflow"
            })),
        )
            .into_response(),
    }
}

/// Delete a workflow
pub async fn delete_workflow(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Verify workflow belongs to current workspace
    match crate::schema::workflows::table
        .inner_join(crate::schema::teams::table)
        .filter(crate::schema::workflows::id.eq(workflow_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .select(Workflow::as_select())
        .first::<Workflow>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Workflow not found or access denied"
                })),
            )
                .into_response();
        }
    }

    match diesel::delete(
        crate::schema::workflows::table.filter(crate::schema::workflows::id.eq(workflow_id)),
    )
    .execute(&mut conn)
    {
        Ok(0) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Workflow not found"
            })),
        )
            .into_response(),
        Ok(_) => {
            let response =
                ApiResponse::success(serde_json::Value::Null, "Workflow deleted successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to delete workflow"
            })),
        )
            .into_response(),
    }
}

/// Get workflow states for a workflow
pub async fn get_workflow_states(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Verify workflow belongs to current workspace
    match crate::schema::workflows::table
        .inner_join(crate::schema::teams::table)
        .filter(crate::schema::workflows::id.eq(workflow_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .select(Workflow::as_select())
        .first::<Workflow>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Workflow not found or access denied"
                })),
            )
                .into_response();
        }
    }

    match crate::schema::workflow_states::table
        .filter(crate::schema::workflow_states::workflow_id.eq(workflow_id))
        .order(crate::schema::workflow_states::category.asc())
        .then_order_by(crate::schema::workflow_states::position.asc())
        .load::<WorkflowState>(&mut conn)
    {
        Ok(states) => {
            let responses: Vec<WorkflowStateResponse> =
                states.into_iter().map(|s| s.into()).collect();
            let response =
                ApiResponse::success(responses, "Workflow states retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to load workflow states"
            })),
        )
            .into_response(),
    }
}

/// Get workflow states for a team's default workflow
pub async fn get_team_default_workflow_states(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    // Verify user has access to the team
    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Check if team belongs to current workspace
    match crate::schema::teams::table
        .filter(crate::schema::teams::id.eq(team_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .first::<crate::db::models::team::Team>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Team not found or access denied"
                })),
            )
                .into_response();
        }
    }

    // Get the default workflow for the team
    let default_workflow = match crate::schema::workflows::table
        .filter(crate::schema::workflows::team_id.eq(team_id))
        .filter(crate::schema::workflows::is_default.eq(true))
        .first::<Workflow>(&mut conn)
    {
        Ok(workflow) => workflow,
        Err(diesel::result::Error::NotFound) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "No default workflow found for this team"
                })),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get default workflow"
                })),
            )
                .into_response();
        }
    };

    // Get workflow states for the default workflow
    match crate::schema::workflow_states::table
        .filter(crate::schema::workflow_states::workflow_id.eq(default_workflow.id))
        .order(crate::schema::workflow_states::category.asc())
        .then_order_by(crate::schema::workflow_states::position.asc())
        .load::<WorkflowState>(&mut conn)
    {
        Ok(states) => {
            let responses: Vec<WorkflowStateResponse> =
                states.into_iter().map(|s| s.into()).collect();
            let response =
                ApiResponse::success(responses, "Default workflow states retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to load workflow states"
            })),
        )
            .into_response(),
    }
}

/// Create a new workflow state
pub async fn create_workflow_state(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(workflow_id): Path<Uuid>,
    Json(request): Json<CreateWorkflowStateRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Verify workflow belongs to current workspace
    match crate::schema::workflows::table
        .inner_join(crate::schema::teams::table)
        .filter(crate::schema::workflows::id.eq(workflow_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .select(Workflow::as_select())
        .first::<Workflow>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Workflow not found or access denied"
                })),
            )
                .into_response();
        }
    }

    let is_default = request.is_default.unwrap_or(false);

    // If this is going to be the default state, unset other defaults in the same category
    if is_default {
        diesel::update(
            crate::schema::workflow_states::table
                .filter(crate::schema::workflow_states::workflow_id.eq(workflow_id))
                .filter(crate::schema::workflow_states::category.eq(request.category.as_str())),
        )
        .set(crate::schema::workflow_states::is_default.eq(false))
        .execute(&mut conn)
        .ok();
    }

    let new_state = NewWorkflowState {
        workflow_id,
        name: request.name,
        description: request.description,
        color: request.color,
        category: request.category,
        position: request.position,
        is_default,
    };

    match diesel::insert_into(crate::schema::workflow_states::table)
        .values(&new_state)
        .get_result::<WorkflowState>(&mut conn)
    {
        Ok(state) => {
            let response: WorkflowStateResponse = state.into();
            let api_response =
                ApiResponse::success(response, "Workflow state created successfully");
            (StatusCode::CREATED, Json(api_response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to create workflow state"
            })),
        )
            .into_response(),
    }
}

/// Create a new workflow state for a team's default workflow
pub async fn create_team_default_workflow_state(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(team_id): Path<Uuid>,
    Json(request): Json<CreateWorkflowStateRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    // Verify user has access to the team
    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Check if team belongs to current workspace
    match crate::schema::teams::table
        .filter(crate::schema::teams::id.eq(team_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .first::<crate::db::models::team::Team>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Team not found or access denied"
                })),
            )
                .into_response();
        }
    }

    // Get the default workflow for the team
    let default_workflow = match crate::schema::workflows::table
        .filter(crate::schema::workflows::team_id.eq(team_id))
        .filter(crate::schema::workflows::is_default.eq(true))
        .first::<Workflow>(&mut conn)
    {
        Ok(workflow) => workflow,
        Err(diesel::result::Error::NotFound) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "No default workflow found for this team"
                })),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get default workflow"
                })),
            )
                .into_response();
        }
    };

    let is_default = request.is_default.unwrap_or(false);

    // If this is going to be the default state, unset other defaults in the same category
    if is_default {
        diesel::update(
            crate::schema::workflow_states::table
                .filter(crate::schema::workflow_states::workflow_id.eq(default_workflow.id))
                .filter(crate::schema::workflow_states::category.eq(request.category.as_str())),
        )
        .set(crate::schema::workflow_states::is_default.eq(false))
        .execute(&mut conn)
        .ok();
    }

    let new_state = NewWorkflowState {
        workflow_id: default_workflow.id,
        name: request.name,
        description: request.description,
        color: request.color,
        category: request.category,
        position: request.position,
        is_default,
    };

    match diesel::insert_into(crate::schema::workflow_states::table)
        .values(&new_state)
        .get_result::<WorkflowState>(&mut conn)
    {
        Ok(state) => {
            let response: WorkflowStateResponse = state.into();
            let api_response =
                ApiResponse::success(response, "Workflow state created successfully");
            (StatusCode::CREATED, Json(api_response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to create workflow state"
            })),
        )
            .into_response(),
    }
}

/// Update a workflow state for a team's default workflow
pub async fn update_team_default_workflow_state(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path((team_id, state_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateWorkflowStateRequest>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    // Verify user has access to the team
    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // Check if team belongs to current workspace
    match crate::schema::teams::table
        .filter(crate::schema::teams::id.eq(team_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .first::<crate::db::models::team::Team>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Team not found or access denied"
                })),
            )
                .into_response();
        }
    }

    // Get the default workflow for the team
    let default_workflow = match crate::schema::workflows::table
        .filter(crate::schema::workflows::team_id.eq(team_id))
        .filter(crate::schema::workflows::is_default.eq(true))
        .first::<Workflow>(&mut conn)
    {
        Ok(workflow) => workflow,
        Err(diesel::result::Error::NotFound) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "No default workflow found for this team"
                })),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get default workflow"
                })),
            )
                .into_response();
        }
    };

    // Verify the workflow state belongs to the default workflow
    match crate::schema::workflow_states::table
        .filter(crate::schema::workflow_states::id.eq(state_id))
        .filter(crate::schema::workflow_states::workflow_id.eq(default_workflow.id))
        .first::<WorkflowState>(&mut conn)
    {
        Ok(_) => {}
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Workflow state not found or access denied"
                })),
            )
                .into_response();
        }
    }

    let mut update_data = UpdateWorkflowState::default();
    if let Some(name) = request.name {
        update_data.name = Some(name);
    }
    if let Some(description) = request.description {
        update_data.description = Some(Some(description));
    }
    if let Some(color) = request.color {
        update_data.color = Some(Some(color));
    }
    if let Some(category) = request.category {
        update_data.category = Some(category);
    }
    if let Some(position) = request.position {
        update_data.position = Some(position);
    }
    if let Some(is_default) = request.is_default {
        update_data.is_default = Some(is_default);

        // If setting as default, unset other defaults in the same category
        if is_default {
            diesel::update(
                crate::schema::workflow_states::table
                    .filter(crate::schema::workflow_states::workflow_id.eq(default_workflow.id))
                    .filter(crate::schema::workflow_states::id.ne(state_id))
                    .filter(
                        crate::schema::workflow_states::category.eq(request
                            .category
                            .as_ref()
                            .map(|c| c.as_str())
                            .unwrap_or("")),
                    ),
            )
            .set(crate::schema::workflow_states::is_default.eq(false))
            .execute(&mut conn)
            .ok();
        }
    }

    match diesel::update(
        crate::schema::workflow_states::table
            .filter(crate::schema::workflow_states::id.eq(state_id)),
    )
    .set(update_data)
    .get_result::<WorkflowState>(&mut conn)
    {
        Ok(state) => {
            let response: WorkflowStateResponse = state.into();
            let api_response =
                ApiResponse::success(response, "Workflow state updated successfully");
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(diesel::result::Error::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Workflow state not found"
            })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to update workflow state"
            })),
        )
            .into_response(),
    }
}

/// Get available workflow transitions for an issue
pub async fn get_issue_transitions(
    State(pool): State<Arc<DbPool>>,
    auth_info: AuthUserInfo,
    Path(issue_id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Database connection failed"
                })),
            )
                .into_response();
        }
    };

    let workspace_id = match auth_info.current_workspace_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No current workspace selected"
                })),
            )
                .into_response();
        }
    };

    // First, get the issue and verify access
    let issue = match crate::schema::issues::table
        .inner_join(crate::schema::teams::table)
        .filter(crate::schema::issues::id.eq(issue_id))
        .filter(crate::schema::teams::workspace_id.eq(workspace_id))
        .select(crate::db::models::issue::Issue::as_select())
        .first::<crate::db::models::issue::Issue>(&mut conn)
    {
        Ok(issue) => issue,
        Err(diesel::result::Error::NotFound) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Issue not found or access denied"
                })),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get issue"
                })),
            )
                .into_response();
        }
    };

    // Get the workflow for this issue
    let workflow_id = match issue.workflow_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Issue does not have an associated workflow"
                })),
            )
                .into_response();
        }
    };

    // Get current workflow state
    let current_state_id = issue.workflow_state_id;

    // Get all workflow states for this workflow
    let workflow_states = match crate::schema::workflow_states::table
        .filter(crate::schema::workflow_states::workflow_id.eq(workflow_id))
        .load::<WorkflowState>(&mut conn)
    {
        Ok(states) => states,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to load workflow states"
                })),
            )
                .into_response();
        }
    };

    // Create a map of state_id -> WorkflowStateResponse for quick lookup
    let state_map: std::collections::HashMap<Uuid, WorkflowStateResponse> = workflow_states
        .into_iter()
        .map(|state| (state.id, state.into()))
        .collect();

    // Get available transitions for this issue
    // Transitions are available if:
    // 1. from_state_id is NULL (can transition from any state)
    // 2. from_state_id matches the current state
    let transitions = match crate::schema::workflow_transitions::table
        .filter(crate::schema::workflow_transitions::workflow_id.eq(workflow_id))
        .filter(
            crate::schema::workflow_transitions::from_state_id
                .is_null()
                .or(crate::schema::workflow_transitions::from_state_id.eq(current_state_id)),
        )
        .load::<WorkflowTransition>(&mut conn)
    {
        Ok(transitions) => transitions,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to load workflow transitions"
                })),
            )
                .into_response();
        }
    };

    // Convert transitions to response format with state information
    let transition_responses: Vec<IssueTransitionResponse> = transitions
        .into_iter()
        .filter_map(|transition| {
            // Get the to_state from our state map
            let to_state = state_map.get(&transition.to_state_id)?.clone();

            // Get the from_state if it exists
            let from_state = transition
                .from_state_id
                .and_then(|id| state_map.get(&id))
                .cloned();

            Some(IssueTransitionResponse {
                id: transition.id,
                workflow_id: transition.workflow_id,
                from_state_id: transition.from_state_id,
                to_state_id: transition.to_state_id,
                name: transition.name,
                description: transition.description,
                created_at: transition.created_at,
                from_state,
                to_state,
            })
        })
        .collect();

    let response = ApiResponse::success(
        transition_responses,
        "Available transitions retrieved successfully",
    );
    (StatusCode::OK, Json(response)).into_response()
}

/// Create default workflow and states for a team
pub fn create_default_workflow_for_team(
    conn: &mut diesel::PgConnection,
    team_id: Uuid,
) -> Result<Workflow, diesel::result::Error> {
    // Create the default workflow
    let new_workflow = NewWorkflow {
        name: "Default Workflow".to_string(),
        description: Some("Default workflow for the team".to_string()),
        team_id,
        is_default: true,
    };

    let workflow = diesel::insert_into(crate::schema::workflows::table)
        .values(&new_workflow)
        .get_result::<Workflow>(conn)?;

    // Create default workflow states
    let default_states = DefaultWorkflowState::get_default_states();
    for state_data in default_states {
        let new_state = NewWorkflowState {
            workflow_id: workflow.id,
            name: state_data.name,
            description: Some(state_data.description),
            color: Some(state_data.color),
            category: state_data.category,
            position: state_data.position,
            is_default: state_data.is_default,
        };

        diesel::insert_into(crate::schema::workflow_states::table)
            .values(&new_state)
            .execute(conn)?;
    }

    Ok(workflow)
}
