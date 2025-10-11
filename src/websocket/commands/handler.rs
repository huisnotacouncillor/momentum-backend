use diesel::{Connection, RunQueryDsl};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::DbPool, error::AppError, services::context::RequestContext,
    websocket::security::SecureMessage,
};

use super::types::*;

#[derive(Clone)]
pub struct WebSocketCommandHandler {
    db: Arc<DbPool>,
    idempotency: IdempotencyControl,
    message_signer: Option<Arc<crate::websocket::MessageSigner>>,
    asset_helper: Arc<crate::utils::AssetUrlHelper>,
}

impl WebSocketCommandHandler {
    pub fn new(db: Arc<DbPool>, asset_helper: Arc<crate::utils::AssetUrlHelper>) -> Self {
        Self {
            db,
            idempotency: IdempotencyControl::new(300),
            message_signer: None,
            asset_helper,
        }
    }

    pub fn with_message_signer(mut self, signer: Arc<crate::websocket::MessageSigner>) -> Self {
        self.message_signer = Some(signer);
        self
    }

    pub fn get_asset_helper(&self) -> Arc<crate::utils::AssetUrlHelper> {
        self.asset_helper.clone()
    }

    async fn verify_secure_message(&self, secure_message: &SecureMessage) -> Result<(), AppError> {
        if let Some(ref signer) = self.message_signer {
            signer
                .verify_message(secure_message)
                .await
                .map_err(|e| AppError::auth(format!("Security verification failed: {}", e)))
        } else {
            Err(AppError::Internal(
                "Message signer not configured".to_string(),
            ))
        }
    }

    #[allow(dead_code)]
    fn generate_idempotency_key(
        &self,
        command: &WebSocketCommand,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        user.user_id.hash(&mut hasher);
        if let Some(workspace_id) = user.current_workspace_id {
            workspace_id.hash(&mut hasher);
        }
        match command {
            WebSocketCommand::CreateLabel { data, .. } => {
                "create_label".hash(&mut hasher);
                data.name.hash(&mut hasher);
                data.color.hash(&mut hasher);
                format!("{:?}", data.level).hash(&mut hasher);
            }
            WebSocketCommand::UpdateLabel { label_id, data, .. } => {
                "update_label".hash(&mut hasher);
                label_id.hash(&mut hasher);
                if let Some(ref name) = data.name {
                    name.hash(&mut hasher);
                }
                if let Some(ref color) = data.color {
                    color.hash(&mut hasher);
                }
                if let Some(ref level) = data.level {
                    format!("{:?}", level).hash(&mut hasher);
                }
            }
            WebSocketCommand::DeleteLabel { label_id, .. } => {
                "delete_label".hash(&mut hasher);
                label_id.hash(&mut hasher);
            }
            WebSocketCommand::QueryLabels { filters, .. } => {
                "query_labels".hash(&mut hasher);
                if let Some(ref level) = filters.level {
                    format!("{:?}", level).hash(&mut hasher);
                }
                if let Some(ref name_pattern) = filters.name_pattern {
                    name_pattern.hash(&mut hasher);
                }
                if let Some(ref color) = filters.color {
                    color.hash(&mut hasher);
                }
                if let Some(limit) = filters.limit {
                    limit.hash(&mut hasher);
                }
                if let Some(offset) = filters.offset {
                    offset.hash(&mut hasher);
                }
            }
            WebSocketCommand::BatchCreateLabels { data, .. } => {
                "batch_create_labels".hash(&mut hasher);
                data.len().hash(&mut hasher);
                for item in data {
                    item.name.hash(&mut hasher);
                    item.color.hash(&mut hasher);
                }
            }
            WebSocketCommand::BatchUpdateLabels { updates, .. } => {
                "batch_update_labels".hash(&mut hasher);
                updates.len().hash(&mut hasher);
                for update in updates {
                    update.label_id.hash(&mut hasher);
                }
            }
            WebSocketCommand::BatchDeleteLabels { label_ids, .. } => {
                "batch_delete_labels".hash(&mut hasher);
                label_ids.len().hash(&mut hasher);
                for label_id in label_ids {
                    label_id.hash(&mut hasher);
                }
            }
            WebSocketCommand::Subscribe { topics, .. } => {
                "subscribe".hash(&mut hasher);
                topics.len().hash(&mut hasher);
                for topic in topics {
                    topic.hash(&mut hasher);
                }
            }
            WebSocketCommand::Unsubscribe { topics, .. } => {
                "unsubscribe".hash(&mut hasher);
                topics.len().hash(&mut hasher);
                for topic in topics {
                    topic.hash(&mut hasher);
                }
            }
            WebSocketCommand::GetConnectionInfo { .. } => {
                "get_connection_info".hash(&mut hasher);
            }
            WebSocketCommand::Ping { .. } => {
                "ping".hash(&mut hasher);
            }
            WebSocketCommand::CreateTeam { data, .. } => {
                "create_team".hash(&mut hasher);
                data.name.hash(&mut hasher);
                data.team_key.hash(&mut hasher);
            }
            WebSocketCommand::UpdateTeam { team_id, data, .. } => {
                "update_team".hash(&mut hasher);
                team_id.hash(&mut hasher);
                if let Some(ref name) = data.name {
                    name.hash(&mut hasher);
                }
                if let Some(ref team_key) = data.team_key {
                    team_key.hash(&mut hasher);
                }
            }
            WebSocketCommand::DeleteTeam { team_id, .. } => {
                "delete_team".hash(&mut hasher);
                team_id.hash(&mut hasher);
            }
            WebSocketCommand::QueryTeams { .. } => {
                "query_teams".hash(&mut hasher);
            }
            WebSocketCommand::AddTeamMember { team_id, data, .. } => {
                "add_team_member".hash(&mut hasher);
                team_id.hash(&mut hasher);
                data.user_id.hash(&mut hasher);
                match data.role {
                    TeamMemberRole::Admin => "admin",
                    TeamMemberRole::Member => "member",
                }
                .hash(&mut hasher);
            }
            WebSocketCommand::UpdateTeamMember {
                team_id,
                member_user_id,
                data,
                ..
            } => {
                "update_team_member".hash(&mut hasher);
                team_id.hash(&mut hasher);
                member_user_id.hash(&mut hasher);
                match data.role {
                    TeamMemberRole::Admin => "admin",
                    TeamMemberRole::Member => "member",
                }
                .hash(&mut hasher);
            }
            WebSocketCommand::RemoveTeamMember {
                team_id,
                member_user_id,
                ..
            } => {
                "remove_team_member".hash(&mut hasher);
                team_id.hash(&mut hasher);
                member_user_id.hash(&mut hasher);
            }
            WebSocketCommand::ListTeamMembers { team_id, .. } => {
                "list_team_members".hash(&mut hasher);
                team_id.hash(&mut hasher);
            }
            WebSocketCommand::InviteWorkspaceMember { data, .. } => {
                "invite_workspace_member".hash(&mut hasher);
                data.email.hash(&mut hasher);
            }
            WebSocketCommand::AcceptInvitation { invitation_id, .. } => {
                "accept_invitation".hash(&mut hasher);
                invitation_id.hash(&mut hasher);
            }
            WebSocketCommand::QueryWorkspaceMembers { filters, .. } => {
                "query_workspace_members".hash(&mut hasher);
                if let Some(user_id) = filters.user_id {
                    user_id.hash(&mut hasher);
                }
                if let Some(ref search) = filters.search {
                    search.hash(&mut hasher);
                }
            }
            WebSocketCommand::CreateWorkspace { data, .. } => {
                "create_workspace".hash(&mut hasher);
                data.name.hash(&mut hasher);
                data.url_key.hash(&mut hasher);
            }
            WebSocketCommand::UpdateWorkspace {
                workspace_id, data, ..
            } => {
                "update_workspace".hash(&mut hasher);
                workspace_id.hash(&mut hasher);
                if let Some(ref name) = data.name {
                    name.hash(&mut hasher);
                }
                if let Some(ref url_key) = data.url_key {
                    url_key.hash(&mut hasher);
                }
            }
            WebSocketCommand::DeleteWorkspace { workspace_id, .. } => {
                "delete_workspace".hash(&mut hasher);
                workspace_id.hash(&mut hasher);
            }
            WebSocketCommand::GetCurrentWorkspace { .. } => {
                "get_current_workspace".hash(&mut hasher);
            }
            WebSocketCommand::CreateProjectStatus { data, .. } => {
                "create_project_status".hash(&mut hasher);
                data.name.hash(&mut hasher);
                data.color.hash(&mut hasher);
                data.category.hash(&mut hasher);
            }
            WebSocketCommand::UpdateProjectStatus {
                status_id, data, ..
            } => {
                "update_project_status".hash(&mut hasher);
                status_id.hash(&mut hasher);
                if let Some(ref name) = data.name {
                    name.hash(&mut hasher);
                }
                if let Some(ref color) = data.color {
                    color.hash(&mut hasher);
                }
                if let Some(ref category) = data.category {
                    category.hash(&mut hasher);
                }
            }
            WebSocketCommand::DeleteProjectStatus { status_id, .. } => {
                "delete_project_status".hash(&mut hasher);
                status_id.hash(&mut hasher);
            }
            WebSocketCommand::QueryProjectStatuses { .. } => {
                "query_project_statuses".hash(&mut hasher);
            }
            WebSocketCommand::GetProjectStatusById { status_id, .. } => {
                "get_project_status_by_id".hash(&mut hasher);
                status_id.hash(&mut hasher);
            }
            WebSocketCommand::UpdateProfile { data, .. } => {
                "update_profile".hash(&mut hasher);
                if let Some(ref name) = data.name {
                    name.hash(&mut hasher);
                }
                if let Some(ref username) = data.username {
                    username.hash(&mut hasher);
                }
                if let Some(ref email) = data.email {
                    email.hash(&mut hasher);
                }
                if let Some(ref avatar_url) = data.avatar_url {
                    avatar_url.hash(&mut hasher);
                }
            }
            WebSocketCommand::CreateProject { data, .. } => {
                "create_project".hash(&mut hasher);
                data.name.hash(&mut hasher);
                data.project_key.hash(&mut hasher);
            }
            WebSocketCommand::UpdateProject {
                project_id, data, ..
            } => {
                "update_project".hash(&mut hasher);
                project_id.hash(&mut hasher);
                if let Some(ref name) = data.name {
                    name.hash(&mut hasher);
                }
                if let Some(ref description) = data.description {
                    description.hash(&mut hasher);
                }
                if let Some(ref priority) = data.priority {
                    priority.hash(&mut hasher);
                }
            }
            WebSocketCommand::DeleteProject { project_id, .. } => {
                "delete_project".hash(&mut hasher);
                project_id.hash(&mut hasher);
            }
            WebSocketCommand::QueryProjects { filters, .. } => {
                "query_projects".hash(&mut hasher);
                if let Some(ref search) = filters.search {
                    search.hash(&mut hasher);
                }
                if let Some(owner_id) = filters.owner_id {
                    owner_id.hash(&mut hasher);
                }
            }
            WebSocketCommand::CreateIssue { data, .. } => {
                "create_issue".hash(&mut hasher);
                data.title.hash(&mut hasher);
                data.team_id.hash(&mut hasher);
                if let Some(ref project_id) = data.project_id {
                    project_id.hash(&mut hasher);
                }
                if let Some(ref assignee_id) = data.assignee_id {
                    assignee_id.hash(&mut hasher);
                }
            }
            WebSocketCommand::UpdateIssue { issue_id, data, .. } => {
                "update_issue".hash(&mut hasher);
                issue_id.hash(&mut hasher);
                if let Some(ref title) = data.title {
                    title.hash(&mut hasher);
                }
                if let Some(team_id) = data.team_id {
                    team_id.hash(&mut hasher);
                }
            }
            WebSocketCommand::DeleteIssue { issue_id, .. } => {
                "delete_issue".hash(&mut hasher);
                issue_id.hash(&mut hasher);
            }
            WebSocketCommand::QueryIssues { filters, .. } => {
                "query_issues".hash(&mut hasher);
                if let Some(team_id) = filters.team_id {
                    team_id.hash(&mut hasher);
                }
                if let Some(project_id) = filters.project_id {
                    project_id.hash(&mut hasher);
                }
                if let Some(assignee_id) = filters.assignee_id {
                    assignee_id.hash(&mut hasher);
                }
                if let Some(ref priority) = filters.priority {
                    priority.hash(&mut hasher);
                }
                if let Some(ref search) = filters.search {
                    search.hash(&mut hasher);
                }
            }
            WebSocketCommand::GetIssue { issue_id, .. } => {
                "get_issue".hash(&mut hasher);
                issue_id.hash(&mut hasher);
            }
        }
        let time_window = chrono::Utc::now().timestamp() / 300;
        time_window.hash(&mut hasher);
        format!("ws_cmd_{:x}", hasher.finish())
    }

    pub async fn handle_secure_command(
        &self,
        secure_message: SecureMessage,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> WebSocketCommandResponse {
        if let Err(e) = self.verify_secure_message(&secure_message).await {
            return WebSocketCommandResponse::error(
                "unknown",
                &secure_message.message_id,
                None,
                WebSocketCommandError::system_error(&format!(
                    "Security verification failed: {}",
                    e
                )),
            );
        }
        let command: WebSocketCommand = match serde_json::from_value(secure_message.payload.clone())
        {
            Ok(cmd) => cmd,
            Err(e) => {
                return WebSocketCommandResponse::error(
                    "unknown",
                    &secure_message.message_id,
                    None,
                    WebSocketCommandError::system_error(&format!("Failed to parse command: {}", e)),
                );
            }
        };
        self.handle_command(command, user).await
    }

    pub async fn handle_command(
        &self,
        command: WebSocketCommand,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> WebSocketCommandResponse {
        let request_id = match &command {
            WebSocketCommand::CreateLabel { request_id, .. }
            | WebSocketCommand::UpdateLabel { request_id, .. }
            | WebSocketCommand::DeleteLabel { request_id, .. }
            | WebSocketCommand::QueryLabels { request_id, .. }
            | WebSocketCommand::BatchCreateLabels { request_id, .. }
            | WebSocketCommand::BatchUpdateLabels { request_id, .. }
            | WebSocketCommand::BatchDeleteLabels { request_id, .. }
            | WebSocketCommand::Subscribe { request_id, .. }
            | WebSocketCommand::Unsubscribe { request_id, .. }
            | WebSocketCommand::GetConnectionInfo { request_id, .. }
            | WebSocketCommand::Ping { request_id, .. }
            | WebSocketCommand::CreateTeam { request_id, .. }
            | WebSocketCommand::UpdateTeam { request_id, .. }
            | WebSocketCommand::DeleteTeam { request_id, .. }
            | WebSocketCommand::QueryTeams { request_id, .. }
            | WebSocketCommand::AddTeamMember { request_id, .. }
            | WebSocketCommand::UpdateTeamMember { request_id, .. }
            | WebSocketCommand::RemoveTeamMember { request_id, .. }
            | WebSocketCommand::ListTeamMembers { request_id, .. }
            | WebSocketCommand::InviteWorkspaceMember { request_id, .. }
            | WebSocketCommand::AcceptInvitation { request_id, .. }
            | WebSocketCommand::QueryWorkspaceMembers { request_id, .. }
            | WebSocketCommand::CreateProjectStatus { request_id, .. }
            | WebSocketCommand::UpdateProjectStatus { request_id, .. }
            | WebSocketCommand::DeleteProjectStatus { request_id, .. }
            | WebSocketCommand::QueryProjectStatuses { request_id, .. }
            | WebSocketCommand::GetProjectStatusById { request_id, .. }
            | WebSocketCommand::CreateWorkspace { request_id, .. }
            | WebSocketCommand::UpdateWorkspace { request_id, .. }
            | WebSocketCommand::DeleteWorkspace { request_id, .. }
            | WebSocketCommand::GetCurrentWorkspace { request_id, .. }
            | WebSocketCommand::UpdateProfile { request_id, .. }
            | WebSocketCommand::CreateProject { request_id, .. }
            | WebSocketCommand::UpdateProject { request_id, .. }
            | WebSocketCommand::DeleteProject { request_id, .. }
            | WebSocketCommand::QueryProjects { request_id, .. }
            | WebSocketCommand::CreateIssue { request_id, .. }
            | WebSocketCommand::UpdateIssue { request_id, .. }
            | WebSocketCommand::DeleteIssue { request_id, .. }
            | WebSocketCommand::QueryIssues { request_id, .. }
            | WebSocketCommand::GetIssue { request_id, .. } => request_id.clone(),
        };

        let idempotency_key = "disabled".to_string();
        let command_type = match &command {
            WebSocketCommand::CreateLabel { .. } => "create_label",
            WebSocketCommand::UpdateLabel { .. } => "update_label",
            WebSocketCommand::DeleteLabel { .. } => "delete_label",
            WebSocketCommand::QueryLabels { .. } => "query_labels",
            WebSocketCommand::BatchCreateLabels { .. } => "batch_create_labels",
            WebSocketCommand::BatchUpdateLabels { .. } => "batch_update_labels",
            WebSocketCommand::BatchDeleteLabels { .. } => "batch_delete_labels",
            WebSocketCommand::Subscribe { .. } => "subscribe",
            WebSocketCommand::Unsubscribe { .. } => "unsubscribe",
            WebSocketCommand::GetConnectionInfo { .. } => "get_connection_info",
            WebSocketCommand::Ping { .. } => "ping",
            WebSocketCommand::CreateTeam { .. } => "create_team",
            WebSocketCommand::UpdateTeam { .. } => "update_team",
            WebSocketCommand::DeleteTeam { .. } => "delete_team",
            WebSocketCommand::QueryTeams { .. } => "query_teams",
            WebSocketCommand::AddTeamMember { .. } => "add_team_member",
            WebSocketCommand::UpdateTeamMember { .. } => "update_team_member",
            WebSocketCommand::RemoveTeamMember { .. } => "remove_team_member",
            WebSocketCommand::ListTeamMembers { .. } => "list_team_members",
            WebSocketCommand::InviteWorkspaceMember { .. } => "invite_workspace_member",
            WebSocketCommand::AcceptInvitation { .. } => "accept_invitation",
            WebSocketCommand::QueryWorkspaceMembers { .. } => "query_workspace_members",
            WebSocketCommand::CreateProjectStatus { .. } => "create_project_status",
            WebSocketCommand::UpdateProjectStatus { .. } => "update_project_status",
            WebSocketCommand::DeleteProjectStatus { .. } => "delete_project_status",
            WebSocketCommand::QueryProjectStatuses { .. } => "query_project_statuses",
            WebSocketCommand::GetProjectStatusById { .. } => "get_project_status_by_id",
            WebSocketCommand::CreateWorkspace { .. } => "create_workspace",
            WebSocketCommand::UpdateWorkspace { .. } => "update_workspace",
            WebSocketCommand::DeleteWorkspace { .. } => "delete_workspace",
            WebSocketCommand::GetCurrentWorkspace { .. } => "get_current_workspace",
            WebSocketCommand::UpdateProfile { .. } => "update_profile",
            WebSocketCommand::CreateProject { .. } => "create_project",
            WebSocketCommand::UpdateProject { .. } => "update_project",
            WebSocketCommand::DeleteProject { .. } => "delete_project",
            WebSocketCommand::QueryProjects { .. } => "query_projects",
            WebSocketCommand::CreateIssue { .. } => "create_issue",
            WebSocketCommand::UpdateIssue { .. } => "update_issue",
            WebSocketCommand::DeleteIssue { .. } => "delete_issue",
            WebSocketCommand::QueryIssues { .. } => "query_issues",
            WebSocketCommand::GetIssue { .. } => "get_issue",
        };

        let workspace_id = match user.current_workspace_id {
            Some(ws) => ws,
            None => {
                return WebSocketCommandResponse::error(
                    command_type,
                    &idempotency_key,
                    request_id,
                    WebSocketCommandError::business_error(
                        "NO_WORKSPACE",
                        "No current workspace selected",
                    ),
                );
            }
        };

        let ctx = RequestContext {
            user_id: user.user_id,
            workspace_id,
            idempotency_key: Some(idempotency_key.clone()),
        };

        let result = match command {
            WebSocketCommand::CreateLabel { data, .. } => self.handle_create_label(ctx, data).await,
            WebSocketCommand::UpdateLabel { label_id, data, .. } => {
                self.handle_update_label(ctx, label_id, data).await
            }
            WebSocketCommand::DeleteLabel { label_id, .. } => {
                self.handle_delete_label(ctx, label_id).await
            }
            WebSocketCommand::QueryLabels { filters, .. } => {
                self.handle_query_labels(ctx, filters).await
            }
            WebSocketCommand::BatchCreateLabels { data, .. } => {
                self.handle_batch_create_labels(ctx, data).await
            }
            WebSocketCommand::BatchUpdateLabels { updates, .. } => {
                self.handle_batch_update_labels(ctx, updates).await
            }
            WebSocketCommand::BatchDeleteLabels { label_ids, .. } => {
                self.handle_batch_delete_labels(ctx, label_ids).await
            }
            WebSocketCommand::Subscribe { topics, .. } => self.handle_subscribe(ctx, topics).await,
            WebSocketCommand::Unsubscribe { topics, .. } => {
                self.handle_unsubscribe(ctx, topics).await
            }
            WebSocketCommand::GetConnectionInfo { .. } => {
                self.handle_get_connection_info(ctx, user).await
            }
            WebSocketCommand::Ping { .. } => Ok(serde_json::json!({"message": "pong"})),
            WebSocketCommand::CreateTeam { data, .. } => self.handle_create_team(ctx, data).await,
            WebSocketCommand::UpdateTeam { team_id, data, .. } => {
                self.handle_update_team(ctx, team_id, data).await
            }
            WebSocketCommand::DeleteTeam { team_id, .. } => {
                self.handle_delete_team(ctx, team_id).await
            }
            WebSocketCommand::QueryTeams { .. } => self.handle_query_teams(ctx).await,
            WebSocketCommand::AddTeamMember { team_id, data, .. } => {
                self.handle_add_team_member(ctx, team_id, data).await
            }
            WebSocketCommand::UpdateTeamMember {
                team_id,
                member_user_id,
                data,
                ..
            } => {
                self.handle_update_team_member(ctx, team_id, member_user_id, data)
                    .await
            }
            WebSocketCommand::RemoveTeamMember {
                team_id,
                member_user_id,
                ..
            } => {
                self.handle_remove_team_member(ctx, team_id, member_user_id)
                    .await
            }
            WebSocketCommand::ListTeamMembers { team_id, .. } => {
                self.handle_list_team_members(ctx, team_id).await
            }
            WebSocketCommand::InviteWorkspaceMember { data, .. } => {
                self.handle_invite_workspace_member(ctx, data).await
            }
            WebSocketCommand::AcceptInvitation { invitation_id, .. } => {
                self.handle_accept_invitation(ctx, invitation_id).await
            }
            WebSocketCommand::QueryWorkspaceMembers { filters, .. } => {
                self.handle_list_workspace_members(ctx, filters).await
            }
            WebSocketCommand::CreateProjectStatus { data, .. } => {
                self.handle_create_project_status(ctx, data).await
            }
            WebSocketCommand::UpdateProjectStatus {
                status_id, data, ..
            } => {
                self.handle_update_project_status(ctx, status_id, data)
                    .await
            }
            WebSocketCommand::DeleteProjectStatus { status_id, .. } => {
                self.handle_delete_project_status(ctx, status_id).await
            }
            WebSocketCommand::QueryProjectStatuses { .. } => {
                self.handle_get_project_statuses(ctx).await
            }
            WebSocketCommand::GetProjectStatusById { status_id, .. } => {
                self.handle_get_project_status_by_id(ctx, status_id).await
            }
            WebSocketCommand::CreateWorkspace { data, .. } => {
                self.handle_create_workspace(ctx, data).await
            }
            WebSocketCommand::UpdateWorkspace {
                workspace_id, data, ..
            } => self.handle_update_workspace(ctx, workspace_id, data).await,
            WebSocketCommand::DeleteWorkspace { workspace_id, .. } => {
                self.handle_delete_workspace(ctx, workspace_id).await
            }
            WebSocketCommand::GetCurrentWorkspace { .. } => {
                self.handle_get_current_workspace(ctx).await
            }
            WebSocketCommand::UpdateProfile { data, .. } => {
                self.handle_update_profile(ctx, data).await
            }
            WebSocketCommand::CreateProject { data, .. } => {
                self.handle_create_project(ctx, data).await
            }
            WebSocketCommand::UpdateProject {
                project_id, data, ..
            } => self.handle_update_project(ctx, project_id, data).await,
            WebSocketCommand::DeleteProject { project_id, .. } => {
                self.handle_delete_project(ctx, project_id).await
            }
            WebSocketCommand::QueryProjects { filters, .. } => {
                self.handle_query_projects(ctx, filters).await
            }
            WebSocketCommand::CreateIssue { data, .. } => self.handle_create_issue(ctx, data).await,
            WebSocketCommand::UpdateIssue { issue_id, data, .. } => {
                self.handle_update_issue(ctx, issue_id, data).await
            }
            WebSocketCommand::DeleteIssue { issue_id, .. } => {
                self.handle_delete_issue(ctx, issue_id).await
            }
            WebSocketCommand::QueryIssues { filters, .. } => {
                self.handle_query_issues(ctx, filters).await
            }
            WebSocketCommand::GetIssue { issue_id, .. } => {
                self.handle_get_issue(ctx, issue_id).await
            }
        };

        match result {
            Ok(data) => {
                WebSocketCommandResponse::success(command_type, &idempotency_key, request_id, data)
            }
            Err(app_error) => WebSocketCommandResponse::error(
                command_type,
                &idempotency_key,
                request_id,
                WebSocketCommandError::business_error("COMMAND_ERROR", &app_error.to_string()),
            ),
        }
    }

    // Label handlers (delegate)
    async fn handle_create_label(
        &self,
        ctx: RequestContext,
        data: CreateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::labels::LabelHandlers::handle_create_label(&self.db, ctx, data).await
    }

    async fn handle_update_label(
        &self,
        ctx: RequestContext,
        label_id: Uuid,
        data: UpdateLabelCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::labels::LabelHandlers::handle_update_label(&self.db, ctx, label_id, data).await
    }

    async fn handle_delete_label(
        &self,
        ctx: RequestContext,
        label_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        super::labels::LabelHandlers::handle_delete_label(&self.db, ctx, label_id).await
    }

    async fn handle_query_labels(
        &self,
        ctx: RequestContext,
        filters: LabelFilters,
    ) -> Result<serde_json::Value, AppError> {
        super::labels::LabelHandlers::handle_query_labels(&self.db, ctx, filters).await
    }

    async fn handle_batch_create_labels(
        &self,
        ctx: RequestContext,
        data: Vec<CreateLabelCommand>,
    ) -> Result<serde_json::Value, AppError> {
        let mut results = Vec::new();
        let mut errors = Vec::new();
        for (index, label_data) in data.into_iter().enumerate() {
            match self.handle_create_label(ctx.clone(), label_data).await {
                Ok(result) => results.push(result),
                Err(e) => errors.push(serde_json::json!({"index": index, "error": e.to_string()})),
            }
        }
        Ok(
            serde_json::json!({"created": results, "errors": errors, "total_created": results.len(), "total_errors": errors.len()}),
        )
    }

    async fn handle_batch_update_labels(
        &self,
        ctx: RequestContext,
        updates: Vec<LabelUpdate>,
    ) -> Result<serde_json::Value, AppError> {
        let mut results = Vec::new();
        let mut errors = Vec::new();
        for (index, update) in updates.into_iter().enumerate() {
            match self.handle_update_label(ctx.clone(), update.label_id, update.data).await {
                Ok(result) => results.push(result),
                Err(e) => errors.push(serde_json::json!({"index": index, "label_id": update.label_id, "error": e.to_string()})),
            }
        }
        Ok(
            serde_json::json!({"updated": results, "errors": errors, "total_updated": results.len(), "total_errors": errors.len()}),
        )
    }

    async fn handle_batch_delete_labels(
        &self,
        ctx: RequestContext,
        label_ids: Vec<Uuid>,
    ) -> Result<serde_json::Value, AppError> {
        let mut results = Vec::new();
        let mut errors = Vec::new();
        for (index, label_id) in label_ids.into_iter().enumerate() {
            match self.handle_delete_label(ctx.clone(), label_id).await {
                Ok(result) => results.push(result),
                Err(e) => errors.push(serde_json::json!({"index": index, "label_id": label_id, "error": e.to_string()})),
            }
        }
        Ok(
            serde_json::json!({"deleted": results, "errors": errors, "total_deleted": results.len(), "total_errors": errors.len()}),
        )
    }

    async fn handle_subscribe(
        &self,
        _ctx: RequestContext,
        topics: Vec<String>,
    ) -> Result<serde_json::Value, AppError> {
        Ok(
            serde_json::json!({"subscribed_topics": topics, "message": "Successfully subscribed to topics"}),
        )
    }

    async fn handle_unsubscribe(
        &self,
        _ctx: RequestContext,
        topics: Vec<String>,
    ) -> Result<serde_json::Value, AppError> {
        Ok(
            serde_json::json!({"unsubscribed_topics": topics, "message": "Successfully unsubscribed from topics"}),
        )
    }

    async fn handle_get_connection_info(
        &self,
        _ctx: RequestContext,
        user: &crate::websocket::auth::AuthenticatedUser,
    ) -> Result<serde_json::Value, AppError> {
        let connection_info = ConnectionInfo {
            user_id: user.user_id,
            username: user.username.clone(),
            connected_at: chrono::Utc::now(),
            last_ping: chrono::Utc::now(),
            subscriptions: vec![],
            message_queue_size: 0,
            state: "connected".to_string(),
        };
        serde_json::to_value(connection_info).map_err(|e| AppError::Internal(e.to_string()))
    }

    // Team handlers
    async fn handle_create_team(
        &self,
        ctx: RequestContext,
        data: CreateTeamCommand,
    ) -> Result<serde_json::Value, AppError> {
        if data.name.trim().is_empty() {
            return Err(AppError::validation("Team name is required"));
        }
        if data.team_key.trim().is_empty() {
            return Err(AppError::validation("Team key is required"));
        }
        if !data
            .team_key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::validation(
                "Team key can only contain letters, numbers, hyphens, and underscores",
            ));
        }
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let req = crate::routes::teams::CreateTeamRequest {
            name: data.name,
            team_key: data.team_key,
            description: data.description,
            icon_url: data.icon_url,
            is_private: data.is_private,
        };
        let user_id = ctx.user_id;
        let current_workspace_id = ctx.workspace_id;
        use crate::schema;
        let result =
            conn.transaction::<crate::db::models::team::Team, diesel::result::Error, _>(|conn| {
                let new_team = crate::db::models::team::NewTeam {
                    name: req.name.clone(),
                    team_key: req.team_key.clone(),
                    description: req.description.clone(),
                    icon_url: req.icon_url.clone(),
                    is_private: req.is_private,
                    workspace_id: current_workspace_id,
                };
                let team: crate::db::models::team::Team = diesel::insert_into(schema::teams::table)
                    .values(&new_team)
                    .get_result::<crate::db::models::team::Team>(conn)?;
                let new_team_member = crate::db::models::team::NewTeamMember {
                    user_id,
                    team_id: team.id,
                    role: "admin".to_string(),
                };
                diesel::insert_into(schema::team_members::table)
                    .values(&new_team_member)
                    .execute(conn)?;
                Ok(team)
            });
        match result {
            Ok(team) => Ok(serde_json::to_value(team).unwrap()),
            Err(e) => {
                if e.to_string().contains("team_key") {
                    Err(AppError::validation(
                        "Team with this key already exists in this workspace",
                    ))
                } else {
                    Err(AppError::Internal("Failed to create team".to_string()))
                }
            }
        }
    }

    async fn handle_update_team(
        &self,
        ctx: RequestContext,
        team_id: Uuid,
        data: UpdateTeamCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        use crate::schema;
        use diesel::prelude::*;
        let existing_team = match schema::teams::table
            .filter(schema::teams::id.eq(team_id))
            .filter(schema::teams::workspace_id.eq(ctx.workspace_id))
            .select(crate::db::models::team::Team::as_select())
            .first::<crate::db::models::team::Team>(&mut conn)
        {
            Ok(team) => team,
            Err(_) => return Err(AppError::not_found("Team not found")),
        };
        if data.name.is_none()
            && data.team_key.is_none()
            && data.description.is_none()
            && data.icon_url.is_none()
            && data.is_private.is_none()
        {
            return Ok(serde_json::to_value(existing_team).unwrap());
        }
        let team_name = data.name.as_ref().unwrap_or(&existing_team.name);
        let team_key_val = data.team_key.as_ref().unwrap_or(&existing_team.team_key);
        let description_val = data
            .description
            .as_ref()
            .or(existing_team.description.as_ref());
        let icon_url_val = data.icon_url.as_ref().or(existing_team.icon_url.as_ref());
        let is_private_val = data.is_private.unwrap_or(existing_team.is_private);
        let updated = diesel::update(schema::teams::table.filter(schema::teams::id.eq(team_id)))
            .set((
                schema::teams::name.eq(team_name),
                schema::teams::team_key.eq(team_key_val),
                schema::teams::description.eq(description_val),
                schema::teams::icon_url.eq(icon_url_val),
                schema::teams::is_private.eq(is_private_val),
            ))
            .get_result::<crate::db::models::team::Team>(&mut conn);
        match updated {
            Ok(team) => Ok(serde_json::to_value(team).unwrap()),
            Err(e) => {
                if e.to_string().contains("team_key") {
                    Err(AppError::validation(
                        "Team with this key already exists in this workspace",
                    ))
                } else {
                    Err(AppError::Internal("Failed to update team".to_string()))
                }
            }
        }
    }

    async fn handle_delete_team(
        &self,
        ctx: RequestContext,
        team_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        use crate::schema;
        use diesel::prelude::*;
        match schema::teams::table
            .filter(schema::teams::id.eq(team_id))
            .filter(schema::teams::workspace_id.eq(ctx.workspace_id))
            .select(crate::db::models::team::Team::as_select())
            .first::<crate::db::models::team::Team>(&mut conn)
        {
            Ok(_) => (),
            Err(_) => return Err(AppError::not_found("Team not found")),
        }
        match diesel::delete(schema::teams::table.filter(schema::teams::id.eq(team_id)))
            .execute(&mut conn)
        {
            Ok(_) => Ok(serde_json::json!({"deleted": true, "team_id": team_id})),
            Err(_) => Err(AppError::Internal("Failed to delete team".to_string())),
        }
    }

    async fn handle_query_teams(&self, ctx: RequestContext) -> Result<serde_json::Value, AppError> {
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        use crate::schema;
        use diesel::prelude::*;
        let list = match schema::teams::table
            .filter(schema::teams::workspace_id.eq(ctx.workspace_id))
            .select(crate::db::models::team::Team::as_select())
            .order(schema::teams::created_at.desc())
            .load::<crate::db::models::team::Team>(&mut conn)
        {
            Ok(list) => list,
            Err(_) => return Err(AppError::Internal("Failed to retrieve teams".to_string())),
        };
        Ok(serde_json::to_value(list).unwrap())
    }

    // Team members via service
    async fn handle_add_team_member(
        &self,
        ctx: RequestContext,
        team_id: Uuid,
        data: AddTeamMemberCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let role_str = match data.role {
            TeamMemberRole::Admin => "admin",
            TeamMemberRole::Member => "member",
        };
        crate::services::team_members_service::TeamMembersService::add(
            &mut conn,
            &ctx,
            team_id,
            data.user_id,
            role_str,
        )?;
        Ok(serde_json::json!({"added": true, "team_id": team_id, "user_id": data.user_id}))
    }

    async fn handle_update_team_member(
        &self,
        ctx: RequestContext,
        team_id: Uuid,
        member_user_id: Uuid,
        data: UpdateTeamMemberCommand,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let role_str = match data.role {
            TeamMemberRole::Admin => "admin",
            TeamMemberRole::Member => "member",
        };
        crate::services::team_members_service::TeamMembersService::update(
            &mut conn,
            &ctx,
            team_id,
            member_user_id,
            role_str,
        )?;
        Ok(serde_json::json!({"updated": true, "team_id": team_id, "user_id": member_user_id}))
    }

    async fn handle_remove_team_member(
        &self,
        ctx: RequestContext,
        team_id: Uuid,
        member_user_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        crate::services::team_members_service::TeamMembersService::remove(
            &mut conn,
            &ctx,
            team_id,
            member_user_id,
        )?;
        Ok(serde_json::json!({"removed": true, "team_id": team_id, "user_id": member_user_id}))
    }

    async fn handle_list_team_members(
        &self,
        ctx: RequestContext,
        team_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let mut conn = self
            .db
            .get()
            .map_err(|_| AppError::Internal("Database connection failed".to_string()))?;
        let members = crate::services::team_members_service::TeamMembersService::list(
            &mut conn, &ctx, team_id,
        )?;
        // Map to DTO-like structure for ws response
        let result: Vec<serde_json::Value> = members
            .into_iter()
            .map(|(member, user)| {
                serde_json::json!({
                    "user": {
                        "id": user.id,
                        "name": user.name,
                        "username": user.username,
                        "email": user.email,
                        "avatar_url": user.avatar_url,
                    },
                    "role": member.role,
                    "joined_at": member.joined_at,
                })
            })
            .collect();
        Ok(serde_json::to_value(result).unwrap())
    }

    // Workspace member handlers (delegate)
    async fn handle_invite_workspace_member(
        &self,
        ctx: RequestContext,
        data: InviteWorkspaceMemberCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::workspace_members::WorkspaceMemberHandlers::handle_invite_member(&self.db, ctx, data)
            .await
    }

    async fn handle_accept_invitation(
        &self,
        ctx: RequestContext,
        invitation_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        super::workspace_members::WorkspaceMemberHandlers::handle_accept_invitation(
            &self.db,
            ctx,
            invitation_id,
        )
        .await
    }

    async fn handle_list_workspace_members(
        &self,
        ctx: RequestContext,
        filters: WorkspaceMemberFilters,
    ) -> Result<serde_json::Value, AppError> {
        super::workspace_members::WorkspaceMemberHandlers::handle_list_workspace_members(
            &self.db,
            &self.asset_helper,
            ctx,
            filters,
        )
        .await
    }

    // Project statuses handlers (delegate)
    async fn handle_get_project_statuses(
        &self,
        ctx: RequestContext,
    ) -> Result<serde_json::Value, AppError> {
        super::project_statuses::ProjectStatusesHandlers::handle_get_list(&self.db, ctx).await
    }

    async fn handle_get_project_status_by_id(
        &self,
        ctx: RequestContext,
        status_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        super::project_statuses::ProjectStatusesHandlers::handle_get_by_id(&self.db, ctx, status_id)
            .await
    }

    async fn handle_create_project_status(
        &self,
        ctx: RequestContext,
        data: CreateProjectStatusCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::project_statuses::ProjectStatusesHandlers::handle_create(&self.db, ctx, data).await
    }

    async fn handle_update_project_status(
        &self,
        ctx: RequestContext,
        status_id: Uuid,
        data: UpdateProjectStatusCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::project_statuses::ProjectStatusesHandlers::handle_update(
            &self.db, ctx, status_id, data,
        )
        .await
    }

    async fn handle_delete_project_status(
        &self,
        ctx: RequestContext,
        status_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        super::project_statuses::ProjectStatusesHandlers::handle_delete(&self.db, ctx, status_id)
            .await
    }

    // Workspace handlers (delegate)
    async fn handle_create_workspace(
        &self,
        ctx: RequestContext,
        data: CreateWorkspaceCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::workspaces::WorkspaceHandlers::handle_create_workspace(
            &self.db,
            ctx,
            data,
            &self.asset_helper,
        )
        .await
    }

    async fn handle_update_workspace(
        &self,
        ctx: RequestContext,
        workspace_id: Uuid,
        data: UpdateWorkspaceCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::workspaces::WorkspaceHandlers::handle_update_workspace(
            &self.db,
            ctx,
            workspace_id,
            data,
            &self.asset_helper,
        )
        .await
    }

    async fn handle_delete_workspace(
        &self,
        ctx: RequestContext,
        workspace_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        super::workspaces::WorkspaceHandlers::handle_delete_workspace(&self.db, ctx, workspace_id)
            .await
    }

    async fn handle_get_current_workspace(
        &self,
        ctx: RequestContext,
    ) -> Result<serde_json::Value, AppError> {
        super::workspaces::WorkspaceHandlers::handle_get_current_workspace(
            &self.db,
            ctx,
            &self.asset_helper,
        )
        .await
    }

    // User profile handlers (delegate)
    async fn handle_update_profile(
        &self,
        ctx: RequestContext,
        data: UpdateProfileCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::user::UserHandlers::handle_update_profile(&self.db, ctx, data, &self.asset_helper)
            .await
    }

    // Project handlers (delegate)
    async fn handle_create_project(
        &self,
        ctx: RequestContext,
        data: CreateProjectCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::projects::ProjectHandlers::handle_create_project(
            &self.db,
            ctx,
            data,
            &self.asset_helper,
        )
        .await
    }

    async fn handle_update_project(
        &self,
        ctx: RequestContext,
        project_id: Uuid,
        data: UpdateProjectCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::projects::ProjectHandlers::handle_update_project(
            &self.db,
            ctx,
            project_id,
            data,
            &self.asset_helper,
        )
        .await
    }

    async fn handle_delete_project(
        &self,
        ctx: RequestContext,
        project_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        super::projects::ProjectHandlers::handle_delete_project(&self.db, ctx, project_id).await
    }

    async fn handle_query_projects(
        &self,
        ctx: RequestContext,
        filters: ProjectFilters,
    ) -> Result<serde_json::Value, AppError> {
        super::projects::ProjectHandlers::handle_query_projects(
            &self.db,
            ctx,
            filters,
            &self.asset_helper,
        )
        .await
    }

    // Issue handlers (delegate to IssueHandlers)
    async fn handle_create_issue(
        &self,
        ctx: RequestContext,
        data: CreateIssueCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::issues::IssueHandlers::handle_create_issue(&self.db, ctx, data).await
    }

    async fn handle_update_issue(
        &self,
        ctx: RequestContext,
        issue_id: Uuid,
        data: UpdateIssueCommand,
    ) -> Result<serde_json::Value, AppError> {
        super::issues::IssueHandlers::handle_update_issue(&self.db, ctx, issue_id, data).await
    }

    async fn handle_delete_issue(
        &self,
        ctx: RequestContext,
        issue_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        super::issues::IssueHandlers::handle_delete_issue(&self.db, ctx, issue_id).await
    }

    async fn handle_query_issues(
        &self,
        ctx: RequestContext,
        filters: IssueFilters,
    ) -> Result<serde_json::Value, AppError> {
        super::issues::IssueHandlers::handle_query_issues(&self.db, ctx, filters).await
    }

    async fn handle_get_issue(
        &self,
        ctx: RequestContext,
        issue_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        super::issues::IssueHandlers::handle_get_issue(&self.db, ctx, issue_id).await
    }

    pub async fn start_cleanup_task(&self) {
        let idempotency = self.idempotency.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                idempotency.cleanup_expired().await;
            }
        });
    }
}
