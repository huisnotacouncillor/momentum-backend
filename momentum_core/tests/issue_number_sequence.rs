//! Tests that issue_number is unique per team.
//!
//! This test verifies that:
//! - Each team has its own independent issue_number sequence starting at 1
//! - Creating issues in different teams maintains separate sequences
//!
//! Note: This test requires DATABASE_URL environment variable and an existing database.

use uuid::Uuid;
use diesel::prelude::*;
use momentum_core::*;

#[tokio::test]
async fn test_issue_number_sequence_per_team() {
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

    // Setup: create Team A
    let team_a_id = Uuid::new_v4();
    diesel::insert_into(schema::teams::table)
        .values((
            schema::teams::id.eq(team_a_id),
            schema::teams::workspace_id.eq(workspace_id),
            schema::teams::name.eq("Team A"),
            schema::teams::team_key.eq("TEAM-A"),
            schema::teams::description.eq(None::<String>),
            schema::teams::icon_url.eq(None::<String>),
            schema::teams::is_private.eq(false),
        ))
        .execute(&mut conn)
        .expect("Failed to insert team A");

    // Setup: create Team B
    let team_b_id = Uuid::new_v4();
    diesel::insert_into(schema::teams::table)
        .values((
            schema::teams::id.eq(team_b_id),
            schema::teams::workspace_id.eq(workspace_id),
            schema::teams::name.eq("Team B"),
            schema::teams::team_key.eq("TEAM-B"),
            schema::teams::description.eq(None::<String>),
            schema::teams::icon_url.eq(None::<String>),
            schema::teams::is_private.eq(false),
        ))
        .execute(&mut conn)
        .expect("Failed to insert team B");

    // Setup: create workflow and state for Team A
    let workflow_a_id = Uuid::new_v4();
    diesel::insert_into(schema::workflows::table)
        .values((
            schema::workflows::id.eq(workflow_a_id),
            schema::workflows::team_id.eq(team_a_id),
            schema::workflows::name.eq("Workflow A"),
            schema::workflows::description.eq(None::<String>),
            schema::workflows::is_default.eq(true),
        ))
        .execute(&mut conn)
        .expect("Failed to insert workflow A");

    let state_a_id = Uuid::new_v4();
    diesel::insert_into(schema::workflow_states::table)
        .values((
            schema::workflow_states::id.eq(state_a_id),
            schema::workflow_states::workflow_id.eq(workflow_a_id),
            schema::workflow_states::name.eq("Todo"),
            schema::workflow_states::description.eq(None::<String>),
            schema::workflow_states::color.eq(None::<String>),
            schema::workflow_states::category.eq("todo"),
            schema::workflow_states::position.eq(0),
            schema::workflow_states::is_default.eq(true),
        ))
        .execute(&mut conn)
        .expect("Failed to insert state A");

    // Setup: create workflow and state for Team B
    let workflow_b_id = Uuid::new_v4();
    diesel::insert_into(schema::workflows::table)
        .values((
            schema::workflows::id.eq(workflow_b_id),
            schema::workflows::team_id.eq(team_b_id),
            schema::workflows::name.eq("Workflow B"),
            schema::workflows::description.eq(None::<String>),
            schema::workflows::is_default.eq(true),
        ))
        .execute(&mut conn)
        .expect("Failed to insert workflow B");

    let state_b_id = Uuid::new_v4();
    diesel::insert_into(schema::workflow_states::table)
        .values((
            schema::workflow_states::id.eq(state_b_id),
            schema::workflow_states::workflow_id.eq(workflow_b_id),
            schema::workflow_states::name.eq("Todo"),
            schema::workflow_states::description.eq(None::<String>),
            schema::workflow_states::color.eq(None::<String>),
            schema::workflow_states::category.eq("todo"),
            schema::workflow_states::position.eq(0),
            schema::workflow_states::is_default.eq(true),
        ))
        .execute(&mut conn)
        .expect("Failed to insert state B");

    // Create context for Team A
    let ctx_a = services::context::RequestContext {
        user_id,
        workspace_id,
        idempotency_key: None,
        trace_id: uuid::Uuid::new_v4().to_string(),
    };

    // Create 3 issues for Team A - verify numbers 1, 2, 3
    let req1 = services::issues::types::CreateIssueRequest {
        title: "Issue A-1".to_string(),
        description: None,
        project_id: None,
        team_id: team_a_id,
        priority: None,
        assignee_id: None,
        reporter_id: None,
        workflow_id: Some(workflow_a_id),
        workflow_state_id: None,
        label_ids: None,
        cycle_id: None,
        parent_issue_id: None,
    };
    let resp1 = services::IssuesService::new()
        .create(&mut conn, &ctx_a, &req1)
        .await
        .expect("Failed to create issue A-1");
    assert_eq!(resp1.issue_number, 1, "First issue in Team A should have number 1");

    let req2 = services::issues::types::CreateIssueRequest {
        title: "Issue A-2".to_string(),
        description: None,
        project_id: None,
        team_id: team_a_id,
        priority: None,
        assignee_id: None,
        reporter_id: None,
        workflow_id: Some(workflow_a_id),
        workflow_state_id: None,
        label_ids: None,
        cycle_id: None,
        parent_issue_id: None,
    };
    let resp2 = services::IssuesService::new()
        .create(&mut conn, &ctx_a, &req2)
        .await
        .expect("Failed to create issue A-2");
    assert_eq!(resp2.issue_number, 2, "Second issue in Team A should have number 2");

    let req3 = services::issues::types::CreateIssueRequest {
        title: "Issue A-3".to_string(),
        description: None,
        project_id: None,
        team_id: team_a_id,
        priority: None,
        assignee_id: None,
        reporter_id: None,
        workflow_id: Some(workflow_a_id),
        workflow_state_id: None,
        label_ids: None,
        cycle_id: None,
        parent_issue_id: None,
    };
    let resp3 = services::IssuesService::new()
        .create(&mut conn, &ctx_a, &req3)
        .await
        .expect("Failed to create issue A-3");
    assert_eq!(resp3.issue_number, 3, "Third issue in Team A should have number 3");

    // Create context for Team B
    let ctx_b = services::context::RequestContext {
        user_id,
        workspace_id,
        idempotency_key: None,
        trace_id: uuid::Uuid::new_v4().to_string(),
    };

    // Create 2 issues for Team B - verify numbers 1, 2 (independent sequence)
    let req4 = services::issues::types::CreateIssueRequest {
        title: "Issue B-1".to_string(),
        description: None,
        project_id: None,
        team_id: team_b_id,
        priority: None,
        assignee_id: None,
        reporter_id: None,
        workflow_id: Some(workflow_b_id),
        workflow_state_id: None,
        label_ids: None,
        cycle_id: None,
        parent_issue_id: None,
    };
    let resp4 = services::IssuesService::new()
        .create(&mut conn, &ctx_b, &req4)
        .await
        .expect("Failed to create issue B-1");
    assert_eq!(resp4.issue_number, 1, "First issue in Team B should have number 1");

    let req5 = services::issues::types::CreateIssueRequest {
        title: "Issue B-2".to_string(),
        description: None,
        project_id: None,
        team_id: team_b_id,
        priority: None,
        assignee_id: None,
        reporter_id: None,
        workflow_id: Some(workflow_b_id),
        workflow_state_id: None,
        label_ids: None,
        cycle_id: None,
        parent_issue_id: None,
    };
    let resp5 = services::IssuesService::new()
        .create(&mut conn, &ctx_b, &req5)
        .await
        .expect("Failed to create issue B-2");
    assert_eq!(resp5.issue_number, 2, "Second issue in Team B should have number 2");

    // Team A creates another issue - verify number 4
    let req6 = services::issues::types::CreateIssueRequest {
        title: "Issue A-4".to_string(),
        description: None,
        project_id: None,
        team_id: team_a_id,
        priority: None,
        assignee_id: None,
        reporter_id: None,
        workflow_id: Some(workflow_a_id),
        workflow_state_id: None,
        label_ids: None,
        cycle_id: None,
        parent_issue_id: None,
    };
    let resp6 = services::IssuesService::new()
        .create(&mut conn, &ctx_a, &req6)
        .await
        .expect("Failed to create issue A-4");
    assert_eq!(resp6.issue_number, 4, "Fourth issue in Team A should have number 4");

    println!("All assertions passed!");
    println!("Team A issue numbers: 1, 2, 3, 4");
    println!("Team B issue numbers: 1, 2");
}