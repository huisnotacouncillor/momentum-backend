//! Tests that create/update Issue returns IssueResponse with relations populated.
//!
//! This test verifies that:
//! - create() returns IssueResponse with team, assignee, labels, workflow_states populated
//! - update() returns IssueResponse with team, assignee, labels, workflow_states populated
//!
//! Note: This test requires DATABASE_URL environment variable and an existing database.

use uuid::Uuid;
use diesel::prelude::*;
use momentum_core::*;

/// Test that IssuesService::create returns IssueResponse with relations populated.
/// This test will fail to compile if create() returns Issue instead of IssueResponse.
#[test]
fn test_create_returns_issue_response_with_relations() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    let manager = diesel::r2d2::ConnectionManager::<PgConnection>::new(database_url);
    let pool = diesel::r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .expect("Failed to create pool");

    let mut conn = pool.get().expect("Failed to get connection");

    let workspace_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Setup: create workspace
    diesel::insert_into(schema::workspaces::table)
        .values((
            schema::workspaces::id.eq(workspace_id),
            schema::workspaces::name.eq("Test Workspace"),
            schema::workspaces::url_key.eq("test-ws"),
        ))
        .execute(&mut conn)
        .expect("Failed to insert workspace");

    // Setup: create user
    diesel::insert_into(schema::users::table)
        .values((
            schema::users::id.eq(user_id),
            schema::users::name.eq("Test User"),
            schema::users::email.eq("test@example.com"),
            schema::users::username.eq("testuser"),
            schema::users::is_active.eq(true),
            schema::users::current_workspace_id.eq(Some(workspace_id)),
        ))
        .execute(&mut conn)
        .expect("Failed to insert user");

    // Setup: create team
    let team_id = Uuid::new_v4();
    diesel::insert_into(schema::teams::table)
        .values((
            schema::teams::id.eq(team_id),
            schema::teams::workspace_id.eq(workspace_id),
            schema::teams::name.eq("Test Team"),
            schema::teams::team_key.eq("TEST"),
            schema::teams::description.eq(None::<String>),
            schema::teams::icon_url.eq(None::<String>),
            schema::teams::is_private.eq(false),
        ))
        .execute(&mut conn)
        .expect("Failed to insert team");

    // Setup: create workflow
    let workflow_id = Uuid::new_v4();
    diesel::insert_into(schema::workflows::table)
        .values((
            schema::workflows::id.eq(workflow_id),
            schema::workflows::team_id.eq(team_id),
            schema::workflows::name.eq("Default Workflow"),
            schema::workflows::description.eq(None::<String>),
            schema::workflows::is_default.eq(true),
        ))
        .execute(&mut conn)
        .expect("Failed to insert workflow");

    // Setup: create workflow state
    let state_id = Uuid::new_v4();
    diesel::insert_into(schema::workflow_states::table)
        .values((
            schema::workflow_states::id.eq(state_id),
            schema::workflow_states::workflow_id.eq(workflow_id),
            schema::workflow_states::name.eq("Todo"),
            schema::workflow_states::description.eq(None::<String>),
            schema::workflow_states::color.eq(None::<String>),
            schema::workflow_states::category.eq("todo"),
            schema::workflow_states::position.eq(0),
            schema::workflow_states::is_default.eq(true),
        ))
        .execute(&mut conn)
        .expect("Failed to insert workflow state");

    // Setup: create label
    let label_id = Uuid::new_v4();
    diesel::insert_into(schema::labels::table)
        .values((
            schema::labels::id.eq(label_id),
            schema::labels::workspace_id.eq(workspace_id),
            schema::labels::name.eq("Test Label"),
            schema::labels::color.eq("#FF0000"),
        ))
        .execute(&mut conn)
        .expect("Failed to insert label");

    // Setup: create assignee user
    let assignee_id = Uuid::new_v4();
    diesel::insert_into(schema::users::table)
        .values((
            schema::users::id.eq(assignee_id),
            schema::users::name.eq("Assignee"),
            schema::users::email.eq("assignee@example.com"),
            schema::users::username.eq("assignee"),
            schema::users::is_active.eq(true),
            schema::users::current_workspace_id.eq(Some(workspace_id)),
        ))
        .execute(&mut conn)
        .expect("Failed to insert assignee");

    let ctx = services::context::RequestContext {
        user_id,
        workspace_id,
        idempotency_key: None,
    };

    // Create issue with labels
    let req = services::issues::types::CreateIssueRequest {
        title: "Test Issue".to_string(),
        description: Some("Test description".to_string()),
        project_id: None,
        team_id,
        priority: None,
        assignee_id: Some(assignee_id),
        reporter_id: None,
        workflow_id: Some(workflow_id),
        workflow_state_id: None,
        label_ids: Some(vec![label_id]),
        cycle_id: None,
        parent_issue_id: None,
    };

    let result = services::IssuesService::new().create(&mut conn, &ctx, &req).await;

    // ASSERT: create should succeed
    assert!(result.is_ok(), "create should succeed: {:?}", result.err());

    let issue_resp = result.unwrap();

    // Verify basic fields
    assert_eq!(issue_resp.title, "Test Issue");
    assert_eq!(issue_resp.team_id, team_id);
    assert_eq!(issue_resp.assignee_id, Some(assignee_id));

    // CRITICAL: Verify relations are populated (not None/empty)
    // These will fail if create() returns Issue instead of IssueResponse
    assert!(issue_resp.team.is_some(), "team should be populated in IssueResponse");
    assert!(issue_resp.team_key.is_some(), "team_key should be populated in IssueResponse");
    assert!(issue_resp.assignee.is_some(), "assignee should be populated in IssueResponse");
    assert!(!issue_resp.workflow_states.is_empty(), "workflow_states should not be empty in IssueResponse");
    assert!(!issue_resp.labels.is_empty(), "labels should not be empty in IssueResponse");

    // Cleanup
    diesel::delete(schema::issue_labels::table.filter(schema::issue_labels::issue_id.eq(issue_resp.id)))
        .execute(&mut conn).ok();
    diesel::delete(schema::issues::table.filter(schema::issues::id.eq(issue_resp.id)))
        .execute(&mut conn).ok();
}

/// Test that IssuesService::update returns IssueResponse with relations populated.
#[test]
fn test_update_returns_issue_response_with_relations() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    let manager = diesel::r2d2::ConnectionManager::<PgConnection>::new(database_url);
    let pool = diesel::r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .expect("Failed to create pool");

    let mut conn = pool.get().expect("Failed to get connection");

    let workspace_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Setup: create workspace
    diesel::insert_into(schema::workspaces::table)
        .values((
            schema::workspaces::id.eq(workspace_id),
            schema::workspaces::name.eq("Test Workspace"),
            schema::workspaces::url_key.eq("test-ws"),
        ))
        .execute(&mut conn)
        .expect("Failed to insert workspace");

    // Setup: create user
    diesel::insert_into(schema::users::table)
        .values((
            schema::users::id.eq(user_id),
            schema::users::name.eq("Test User"),
            schema::users::email.eq("test@example.com"),
            schema::users::username.eq("testuser"),
            schema::users::is_active.eq(true),
            schema::users::current_workspace_id.eq(Some(workspace_id)),
        ))
        .execute(&mut conn)
        .expect("Failed to insert user");

    // Setup: create team
    let team_id = Uuid::new_v4();
    diesel::insert_into(schema::teams::table)
        .values((
            schema::teams::id.eq(team_id),
            schema::teams::workspace_id.eq(workspace_id),
            schema::teams::name.eq("Test Team"),
            schema::teams::team_key.eq("TEST"),
            schema::teams::description.eq(None::<String>),
            schema::teams::icon_url.eq(None::<String>),
            schema::teams::is_private.eq(false),
        ))
        .execute(&mut conn)
        .expect("Failed to insert team");

    // Setup: create workflow
    let workflow_id = Uuid::new_v4();
    diesel::insert_into(schema::workflows::table)
        .values((
            schema::workflows::id.eq(workflow_id),
            schema::workflows::team_id.eq(team_id),
            schema::workflows::name.eq("Default Workflow"),
            schema::workflows::description.eq(None::<String>),
            schema::workflows::is_default.eq(true),
        ))
        .execute(&mut conn)
        .expect("Failed to insert workflow");

    // Setup: create workflow state
    let state_id = Uuid::new_v4();
    diesel::insert_into(schema::workflow_states::table)
        .values((
            schema::workflow_states::id.eq(state_id),
            schema::workflow_states::workflow_id.eq(workflow_id),
            schema::workflow_states::name.eq("Todo"),
            schema::workflow_states::description.eq(None::<String>),
            schema::workflow_states::color.eq(None::<String>),
            schema::workflow_states::category.eq("todo"),
            schema::workflow_states::position.eq(0),
            schema::workflow_states::is_default.eq(true),
        ))
        .execute(&mut conn)
        .expect("Failed to insert workflow state");

    // Setup: create label
    let label_id = Uuid::new_v4();
    diesel::insert_into(schema::labels::table)
        .values((
            schema::labels::id.eq(label_id),
            schema::labels::workspace_id.eq(workspace_id),
            schema::labels::name.eq("Test Label"),
            schema::labels::color.eq("#FF0000"),
        ))
        .execute(&mut conn)
        .expect("Failed to insert label");

    // Setup: create assignee user
    let assignee_id = Uuid::new_v4();
    diesel::insert_into(schema::users::table)
        .values((
            schema::users::id.eq(assignee_id),
            schema::users::name.eq("Assignee"),
            schema::users::email.eq("assignee@example.com"),
            schema::users::username.eq("assignee"),
            schema::users::is_active.eq(true),
            schema::users::current_workspace_id.eq(Some(workspace_id)),
        ))
        .execute(&mut conn)
        .expect("Failed to insert assignee");

    let ctx = services::context::RequestContext {
        user_id,
        workspace_id,
        idempotency_key: None,
    };

    // First create an issue without labels
    let create_req = services::issues::types::CreateIssueRequest {
        title: "Original Title".to_string(),
        description: None,
        project_id: None,
        team_id,
        priority: None,
        assignee_id: None,
        reporter_id: None,
        workflow_id: Some(workflow_id),
        workflow_state_id: None,
        label_ids: None,
        cycle_id: None,
        parent_issue_id: None,
    };

    let created = services::IssuesService::new().create(&mut conn, &ctx, &create_req).await.unwrap();

    // Update the issue with new title, assignee, and labels
    let update_req = services::issues::types::UpdateIssueRequest {
        title: Some("Updated Title".to_string()),
        description: Some("Updated desc".to_string()),
        project_id: None,
        team_id: None,
        priority: None,
        assignee_id: Some(assignee_id),
        reporter_id: None,
        workflow_id: None,
        workflow_state_id: None,
        cycle_id: None,
        label_ids: Some(vec![label_id]),
    };

    let result = services::IssuesService::new().update(&mut conn, &ctx, created.id, &update_req).await;

    // ASSERT: update should succeed
    assert!(result.is_ok(), "update should succeed: {:?}", result.err());

    let issue_resp = result.unwrap();

    // Verify updated fields
    assert_eq!(issue_resp.title, "Updated Title");
    assert_eq!(issue_resp.assignee_id, Some(assignee_id));

    // CRITICAL: Verify relations are populated
    assert!(issue_resp.team.is_some(), "team should be populated in IssueResponse");
    assert!(issue_resp.team_key.is_some(), "team_key should be populated in IssueResponse");
    assert!(issue_resp.assignee.is_some(), "assignee should be populated in IssueResponse");
    assert!(!issue_resp.workflow_states.is_empty(), "workflow_states should not be empty in IssueResponse");
    assert!(!issue_resp.labels.is_empty(), "labels should not be empty in IssueResponse");

    // Cleanup
    diesel::delete(schema::issue_labels::table.filter(schema::issue_labels::issue_id.eq(issue_resp.id)))
        .execute(&mut conn).ok();
    diesel::delete(schema::issues::table.filter(schema::issues::id.eq(issue_resp.id)))
        .execute(&mut conn).ok();
}