pub mod auth;
pub mod issues;
pub mod labels;
pub mod projects;
pub mod teams;
pub mod users;
pub mod workspaces;
pub mod workspace_members;

use crate::AppState;
use axum::{
    Router,
    routing::{get, post, put, delete},
};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    // Create a router for routes that need the full AppState (including Redis)
    let app_routes = Router::new()
        .route("/labels", get(labels::get_labels))
        .route("/labels", post(labels::create_label))
        .route("/labels/:label_id", put(labels::update_label))
        .route("/labels/:label_id", delete(labels::delete_label))
        .route("/auth/switch-workspace", post(auth::switch_workspace))
        .route("/workspaces", post(workspaces::create_workspace))
        .route("/workspaces/current", get(workspaces::get_current_workspace))
        .route("/workspaces/:workspace_id", put(workspaces::update_workspace))
        .route("/workspaces/:workspace_id", delete(workspaces::delete_workspace))
        .route("/workspace-members", get(workspace_members::get_current_workspace_members))
        .route("/workspaces/:workspace_id/members", get(workspace_members::get_workspace_members))
        .route("/issues", post(issues::create_issue))
        .route("/issues", get(issues::get_issues))
        .route("/issues/:issue_id", get(issues::get_issue_by_id))
        .route("/issues/:issue_id", put(issues::update_issue))
        .route("/issues/:issue_id", delete(issues::delete_issue))
        .route("/users/profile", put(users::update_profile))
        .with_state(state.clone());

    // Create a router for routes that only need the database pool
    // Note: Auth routes are handled in main.rs to avoid middleware conflicts
    let db_routes = Router::new()
        .route("/users", get(crate::routes::users::get_users))
        .route("/auth/profile", get(auth::get_profile))
        .route(
            "/auth/oauth/:provider/authorize",
            get(auth::oauth_authorize),
        )
        .route("/auth/oauth/:provider/callback", get(auth::oauth_callback))
        .route("/projects", post(projects::create_project))
        .route("/projects", get(projects::get_projects))
        .route("/issues/priorities", get(issues::get_issue_priorities))
        .route("/teams", post(teams::create_team))
        .route("/teams", get(teams::get_teams))
        .route("/teams/:team_id", get(teams::get_team))
        .route("/teams/:team_id", put(teams::update_team))
        .route("/teams/:team_id", delete(teams::delete_team))
        .route("/teams/:team_id/members", post(teams::add_team_member))
        .route("/teams/:team_id/members", get(teams::get_team_members_list))
        .route("/teams/:team_id/members/:user_id", put(teams::update_team_member))
        .route("/teams/:team_id/members/:user_id", delete(teams::remove_team_member))
        .route("/user/teams", get(teams::get_user_teams))
        .with_state(Arc::new(state.db.clone()));

    // Merge the routers
    app_routes.merge(db_routes)
}