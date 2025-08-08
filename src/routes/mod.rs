pub mod auth;
pub mod issues;
pub mod labels;
pub mod projects;
pub mod teams;
pub mod users;

use crate::{db::DbPool, websocket};
use axum::{
    Router,
    routing::{get, post, put, delete},
};
use std::sync::Arc;

pub fn create_router(pool: DbPool) -> Router {
    let db_arc = Arc::new(pool);

    // Create WebSocket state
    let ws_state = websocket::create_websocket_state(db_arc.clone());

    // Create main router with auth and user routes
    let main_router = Router::new()
        .route("/users", get(crate::routes::users::get_users))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh_token))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/profile", get(auth::get_profile))
        .route("/auth/switch-workspace", post(auth::switch_workspace))
        .route(
            "/auth/oauth/:provider/authorize",
            get(auth::oauth_authorize),
        )
        .route("/auth/oauth/:provider/callback", get(auth::oauth_callback))
        .route("/projects", post(projects::create_project))
        .route("/projects", get(projects::get_projects))
        .route("/issues/priorities", get(issues::get_issue_priorities))
        .route("/labels", get(labels::get_labels))
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
        .with_state(db_arc);

    // Create WebSocket router
    let ws_router = websocket::create_websocket_routes().with_state(ws_state);

    // Merge routers
    main_router.merge(ws_router)
}