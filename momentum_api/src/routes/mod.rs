pub mod auth;
pub mod automation;
pub mod comments;
pub mod cycles;
pub mod health;
pub mod invitations;
pub mod issues;
pub mod labels;
pub mod notifications;
pub mod oauth;
pub mod plugins;
pub mod project_statuses;
pub mod projects;
pub mod refresh_token_store;
pub mod teams;
pub mod users;
pub mod workflows;
pub mod workspace_members;
pub mod workspaces;

use crate::state::AppState;
use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    // Create a router for routes that need the full AppState (including Redis)
    let app_routes = Router::new()
        .route("/labels", get(labels::get_labels))
        .route("/labels", post(labels::create_label))
        .route("/labels/:label_id", put(labels::update_label))
        .route("/labels/:label_id", delete(labels::delete_label))
        .route("/notifications", get(notifications::get_notifications))
        .route("/notifications/:notification_id/read", patch(notifications::mark_notification_read))
        .route("/notifications/read-all", post(notifications::mark_all_notifications_read))
        .route("/notifications/unread-count", get(notifications::get_unread_count))
        .route("/auth/profile", get(auth::get_profile))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/switch-workspace", post(auth::switch_workspace))
        .route("/auth/refresh", post(auth::refresh_token))
        .route("/workspaces", post(workspaces::create_workspace))
        .route(
            "/workspaces/current",
            get(workspaces::get_current_workspace),
        )
        .route(
            "/workspaces/:workspace_id",
            put(workspaces::update_workspace),
        )
        .route(
            "/workspaces/:workspace_id",
            delete(workspaces::delete_workspace),
        )
        .route(
            "/workspace-members",
            get(workspace_members::get_current_workspace_members),
        )
        .route(
            "/workspace-member-and-invitations",
            get(workspace_members::get_workspace_members_and_invitations),
        )
        .route(
            "/workspaces/:workspace_id/members",
            get(workspace_members::get_workspace_members),
        )
        .route("/invitations", post(invitations::invite_member))
        .route("/invitations", get(invitations::get_user_invitations))
        .route(
            "/invitations/:invitation_id",
            get(invitations::get_invitation_by_id),
        )
        .route(
            "/invitations/:invitation_id/accept",
            post(invitations::accept_invitation),
        )
        .route(
            "/invitations/:invitation_id/decline",
            post(invitations::decline_invitation),
        )
        .route(
            "/invitations/:invitation_id/revoke",
            post(invitations::revoke_invitation),
        )
        .route("/issues", post(issues::create_issue))
        .route("/issues", get(issues::get_issues))
        .route("/issues/:issue_id", get(issues::get_issue))
        .route("/issues/:issue_id", put(issues::update_issue))
        .route("/issues/:issue_id", delete(issues::delete_issue))
        .route("/issues/:issue_id/comments", get(comments::get_comments))
        .route("/issues/:issue_id/comments", post(comments::create_comment))
        .route("/comments/:comment_id", get(comments::get_comment))
        .route("/comments/:comment_id", put(comments::update_comment))
        .route("/comments/:comment_id", delete(comments::delete_comment))
        .route("/users/profile", put(users::update_profile))
        .route("/projects", get(projects::get_projects))
        .route("/projects", post(projects::create_project))
        .route("/projects/:project_id", put(projects::update_project))
        .route("/projects/:project_id", delete(projects::delete_project))
        .route("/cycles", post(cycles::create_cycle))
        .route("/cycles", get(cycles::get_cycles))
        .route("/cycles/:cycle_id", get(cycles::get_cycle_by_id))
        .route("/cycles/:cycle_id", put(cycles::update_cycle))
        .route("/cycles/:cycle_id", delete(cycles::delete_cycle))
        .route("/cycles/:cycle_id/stats", get(cycles::get_cycle_stats))
        .route("/cycles/:cycle_id/issues", get(cycles::get_cycle_issues))
        .route(
            "/cycles/:cycle_id/issues",
            post(cycles::assign_issues_to_cycle),
        )
        .route(
            "/cycles/:cycle_id/issues",
            delete(cycles::remove_issues_from_cycle),
        )
        .route(
            "/cycles/auto-update-status",
            post(cycles::update_cycle_status_auto),
        )
        .route(
            "/project-statuses",
            post(project_statuses::create_project_status),
        )
        .route(
            "/project-statuses",
            get(project_statuses::get_project_statuses),
        )
        .route(
            "/project-statuses/:status_id",
            get(project_statuses::get_project_status_by_id),
        )
        .route(
            "/project-statuses/:status_id",
            put(project_statuses::update_project_status),
        )
        .route(
            "/project-statuses/:status_id",
            delete(project_statuses::delete_project_status),
        )
        .route("/teams/:team_id/workflows", get(workflows::get_workflows))
        .route(
            "/teams/:team_id/workflows",
            post(workflows::create_workflow),
        )
        .route(
            "/teams/:team_id/workflows/default/states",
            get(workflows::get_team_default_workflow_states),
        )
        .route(
            "/teams/:team_id/workflows/default/states",
            post(workflows::create_team_default_workflow_state),
        )
        .route(
            "/teams/:team_id/workflows/default/states/:state_id",
            put(workflows::update_team_default_workflow_state),
        )
        .route(
            "/workflows/:workflow_id",
            get(workflows::get_workflow_by_id),
        )
        .route("/workflows/:workflow_id", put(workflows::update_workflow))
        .route(
            "/workflows/:workflow_id",
            delete(workflows::delete_workflow),
        )
        .route(
            "/workflows/:workflow_id/states",
            get(workflows::get_workflow_states),
        )
        .route(
            "/workflows/:workflow_id/states",
            post(workflows::create_workflow_state),
        )
        .route(
            "/issues/:issue_id/transitions",
            get(workflows::get_issue_transitions),
        )
        // Plugin routes
        .route("/plugins", get(plugins::list_plugins))
        .route("/plugins/install", post(plugins::install_plugin))
        .route("/plugins/:inst_id/enable", post(plugins::enable_plugin))
        .route("/plugins/:inst_id/disable", post(plugins::disable_plugin))
        .route(
            "/workspaces/:workspace_id/plugins",
            get(plugins::list_workspace_plugins),
        )
        .route(
            "/workspaces/:workspace_id/plugins/:plugin_id",
            delete(plugins::uninstall_plugin),
        )
        .route(
            "/workspaces/:workspace_id/fields",
            get(plugins::list_workspace_fields),
        )
        // Automation rules routes
        .route(
            "/workspaces/:workspace_id/automation-rules",
            get(automation::list_rules),
        )
        .route(
            "/workspaces/:workspace_id/automation-rules",
            post(automation::create_rule),
        )
        .route(
            "/workspaces/:workspace_id/automation-rules/:rule_id",
            get(automation::get_rule),
        )
        .route(
            "/workspaces/:workspace_id/automation-rules/:rule_id",
            patch(automation::update_rule),
        )
        .route(
            "/workspaces/:workspace_id/automation-rules/:rule_id",
            delete(automation::delete_rule),
        )
        .with_state(state.clone());

    // Create a router for routes that only need the database pool
    // Note: Auth routes are handled in main.rs to avoid middleware conflicts
    let db_routes = Router::new()
        .route("/teams", post(teams::create_team))
        .route("/teams", get(teams::get_teams))
        .route("/teams/:team_id", get(teams::get_team))
        .route("/teams/:team_id", put(teams::update_team))
        .route("/teams/:team_id", delete(teams::delete_team))
        .route("/teams/:team_id/members", post(teams::add_team_member))
        .route("/teams/:team_id/members", get(teams::get_team_members_list))
        .route(
            "/teams/:team_id/members/:user_id",
            put(teams::update_team_member),
        )
        .route(
            "/teams/:team_id/members/:user_id",
            delete(teams::remove_team_member),
        )
        .route("/user/teams", get(teams::get_user_teams))
        .with_state(Arc::new(state.db.clone()));

    // Merge the routers
    let router = app_routes.merge(db_routes);

    // P0 修复：添加健康检查端点（无需认证）
    // P3.1 修复：添加 /v1 路径前缀以支持 API 版本化
    // P3.2 修复：添加 Prometheus 指标导出端点
    router
        .route("/health", get(health::health))
        .route("/live", get(health::liveness))
        .route("/ready", get(health::readiness))
        .route("/metrics", get(crate::observability::prometheus_handler))
        .with_state(state.clone())
}

/// P3.1 修复：API 版本化路由
///
/// 当前所有业务 API 都通过 /v1 前缀暴露。
/// 未来添加 /v2 时，不会破坏现有客户端。
///
/// ## 版本策略
///
/// - `/v1/*` - 当前稳定版本
/// - `/v2/*` - 未来添加
/// - 不带版本前缀的（如 /health、/metrics）- 系统级端点
///
/// ## 迁移指南
///
/// 新客户端应使用 `/v1/workspaces/:id` 而不是 `/workspaces/:id`
pub fn create_v1_router(state: Arc<AppState>) -> axum::Router {
    create_router(state)
}
