use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use momentum_core::db::enums::LabelLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSocketCommand {
    CreateLabel {
        data: CreateLabelCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    UpdateLabel {
        label_id: Uuid,
        data: UpdateLabelCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    DeleteLabel {
        label_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    GetLabel {
        label_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    QueryLabels {
        filters: LabelFilters,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    BatchCreateLabels {
        data: Vec<CreateLabelCommand>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    BatchUpdateLabels {
        updates: Vec<LabelUpdate>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    BatchDeleteLabels {
        label_ids: Vec<Uuid>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    Subscribe {
        topics: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    Unsubscribe {
        topics: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    GetConnectionInfo {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    Ping {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    GetFeatureFlags {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    // Team
    CreateTeam {
        data: CreateTeamCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    UpdateTeam {
        team_id: Uuid,
        data: UpdateTeamCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    DeleteTeam {
        team_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    QueryTeams {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    // Team members
    AddTeamMember {
        team_id: Uuid,
        data: AddTeamMemberCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    UpdateTeamMember {
        team_id: Uuid,
        member_user_id: Uuid,
        data: UpdateTeamMemberCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    RemoveTeamMember {
        team_id: Uuid,
        member_user_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    ListTeamMembers {
        team_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    // Workspace members
    InviteWorkspaceMember {
        data: InviteWorkspaceMemberCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    AcceptInvitation {
        invitation_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    GetInvitation {
        invitation_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    QueryWorkspaceMembers {
        filters: WorkspaceMemberFilters,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    // Workspace
    CreateWorkspace {
        data: CreateWorkspaceCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    UpdateWorkspace {
        workspace_id: Uuid,
        data: UpdateWorkspaceCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    DeleteWorkspace {
        workspace_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    GetCurrentWorkspace {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    // Project statuses
    CreateProjectStatus {
        data: CreateProjectStatusCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    UpdateProjectStatus {
        status_id: Uuid,
        data: UpdateProjectStatusCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    DeleteProjectStatus {
        status_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    QueryProjectStatuses {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    GetProjectStatusById {
        status_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    // User profile
    UpdateProfile {
        data: UpdateProfileCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    // Projects
    CreateProject {
        data: CreateProjectCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    UpdateProject {
        project_id: Uuid,
        data: UpdateProjectCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    DeleteProject {
        project_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    GetProject {
        project_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    QueryProjects {
        filters: ProjectFilters,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    // Issues
    CreateIssue {
        data: CreateIssueCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    UpdateIssue {
        issue_id: Uuid,
        data: UpdateIssueCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    DeleteIssue {
        issue_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    QueryIssues {
        filters: IssueFilters,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    GetIssue {
        issue_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    // Cycles
    QueryCycles {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    GetCycle {
        cycle_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    CreateCycle {
        data: CreateCycleCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    UpdateCycle {
        cycle_id: Uuid,
        data: UpdateCycleCommand,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    DeleteCycle {
        cycle_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLabelCommand {
    pub name: String,
    pub color: String,
    pub level: LabelLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLabelCommand {
    pub name: Option<String>,
    pub color: Option<String>,
    pub level: Option<LabelLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelFilters {
    pub workspace_id: Option<Uuid>,
    pub level: Option<LabelLevel>,
    pub name_pattern: Option<String>,
    pub color: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelUpdate {
    pub label_id: Uuid,
    pub data: UpdateLabelCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub user_id: Uuid,
    pub username: String,
    pub connected_at: DateTime<Utc>,
    pub last_ping: DateTime<Utc>,
    pub subscriptions: Vec<String>,
    pub message_queue_size: usize,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketCommandResponse {
    pub command_type: String,
    pub idempotency_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WebSocketCommandError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<WebSocketResponseMeta>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketResponseMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<WebSocketPagination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_stats: Option<WebSocketBatchStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketPagination {
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketBatchStats {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
    pub skipped: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketCommandError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
}

impl WebSocketCommandResponse {
    pub fn success(
        command_type: &str,
        idempotency_key: &str,
        request_id: Option<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            command_type: command_type.to_string(),
            idempotency_key: idempotency_key.to_string(),
            request_id,
            success: true,
            data: Some(data),
            error: None,
            meta: None,
            timestamp: Utc::now(),
        }
    }

    pub fn success_with_meta(
        command_type: &str,
        idempotency_key: &str,
        request_id: Option<String>,
        data: serde_json::Value,
        meta: WebSocketResponseMeta,
    ) -> Self {
        Self {
            command_type: command_type.to_string(),
            idempotency_key: idempotency_key.to_string(),
            request_id,
            success: true,
            data: Some(data),
            error: None,
            meta: Some(meta),
            timestamp: Utc::now(),
        }
    }

    pub fn error(
        command_type: &str,
        idempotency_key: &str,
        request_id: Option<String>,
        error: WebSocketCommandError,
    ) -> Self {
        Self {
            command_type: command_type.to_string(),
            idempotency_key: idempotency_key.to_string(),
            request_id,
            success: false,
            data: None,
            error: Some(error),
            meta: None,
            timestamp: Utc::now(),
        }
    }

    pub fn ok(
        command_type: &str,
        idempotency_key: &str,
        request_id: Option<String>,
        message: &str,
    ) -> Self {
        Self {
            command_type: command_type.to_string(),
            idempotency_key: idempotency_key.to_string(),
            request_id,
            success: true,
            data: Some(serde_json::json!({"message": message})),
            error: None,
            meta: None,
            timestamp: Utc::now(),
        }
    }
}

impl WebSocketCommandError {
    pub fn validation_error(field: &str, message: &str) -> Self {
        Self {
            code: "VALIDATION_ERROR".to_string(),
            message: message.to_string(),
            field: Some(field.to_string()),
            details: None,
            error_type: Some("validation".to_string()),
        }
    }

    pub fn business_error(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field: None,
            details: None,
            error_type: Some("business".to_string()),
        }
    }

    pub fn system_error(message: &str) -> Self {
        Self {
            code: "SYSTEM_ERROR".to_string(),
            message: message.to_string(),
            field: None,
            details: None,
            error_type: Some("system".to_string()),
        }
    }

    pub fn permission_error(message: &str) -> Self {
        Self {
            code: "PERMISSION_ERROR".to_string(),
            message: message.to_string(),
            field: None,
            details: None,
            error_type: Some("permission".to_string()),
        }
    }

    pub fn not_found(resource: &str) -> Self {
        Self {
            code: "NOT_FOUND".to_string(),
            message: format!("{} not found", resource),
            field: None,
            details: None,
            error_type: Some("not_found".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IdempotencyControl {
    processed_commands: Arc<RwLock<HashMap<String, WebSocketCommandResponse>>>,
    expiration_seconds: u64,
}

impl IdempotencyControl {
    pub fn new(expiration_seconds: u64) -> Self {
        Self {
            processed_commands: Arc::new(RwLock::new(HashMap::new())),
            expiration_seconds,
        }
    }

    pub async fn is_processed(&self, idempotency_key: &str) -> Option<WebSocketCommandResponse> {
        let commands = self.processed_commands.read().await;
        commands.get(idempotency_key).cloned()
    }

    pub async fn mark_processed(
        &self,
        idempotency_key: String,
        response: WebSocketCommandResponse,
    ) {
        let mut commands = self.processed_commands.write().await;
        commands.insert(idempotency_key, response);
    }

    pub async fn cleanup_expired(&self) {
        let cutoff_time = Utc::now() - chrono::Duration::seconds(self.expiration_seconds as i64);
        let mut commands = self.processed_commands.write().await;
        commands.retain(|_, response| response.timestamp > cutoff_time);
    }
}

// Team command payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTeamCommand {
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTeamCommand {
    pub name: Option<String>,
    pub team_key: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_private: Option<bool>,
}

// Team member command payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamMemberRole {
    Admin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTeamMemberCommand {
    pub user_id: Uuid,
    pub role: TeamMemberRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTeamMemberCommand {
    pub role: TeamMemberRole,
}

// Workspace member command payloads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMemberRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteWorkspaceMemberCommand {
    pub email: String,
    pub role: WorkspaceMemberRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMemberFilters {
    pub role: Option<WorkspaceMemberRole>,
    pub user_id: Option<Uuid>,
    pub search: Option<String>,
}

// Workspace command payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceCommand {
    pub name: String,
    pub url_key: String,
    pub logo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceCommand {
    pub name: Option<String>,
    pub url_key: Option<String>,
    pub logo_url: Option<String>,
}

// Project status command payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectStatusCommand {
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    // one of: backlog, planned, in_progress, completed, canceled
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectStatusCommand {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    // one of: backlog, planned, in_progress, completed, canceled
    pub category: Option<String>,
}

// User profile command payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileCommand {
    pub name: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

// Project command payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectCommand {
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub target_date: Option<chrono::NaiveDate>,
    pub project_status_id: Option<Uuid>,
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectCommand {
    pub name: Option<String>,
    pub description: Option<String>,
    pub target_date: Option<chrono::NaiveDate>,
    pub project_status_id: Option<Uuid>,
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFilters {
    pub search: Option<String>,
    pub owner_id: Option<Uuid>,
    pub team_id: Option<Uuid>,  // Frontend expects this but ProjectsService doesn't use it
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

// Issue command payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIssueCommand {
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Uuid,
    pub priority: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
    pub cycle_id: Option<Uuid>,
    pub parent_issue_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIssueCommand {
    pub title: Option<String>,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub priority: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub workflow_state_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub label_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueFilters {
    #[serde(default, deserialize_with = "deserialize_optional_uuid")]
    pub team_id: Option<Uuid>,
    #[serde(default, deserialize_with = "deserialize_optional_uuid")]
    pub project_id: Option<Uuid>,
    #[serde(default, deserialize_with = "deserialize_optional_uuid")]
    pub assignee_id: Option<Uuid>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub priority: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub search: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub cursor: Option<String>,
}

// Cycle commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCycleCommand {
    pub team_id: Uuid,
    pub name: String,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub description: Option<String>,
    pub goal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCycleCommand {
    pub name: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub goal: Option<String>,
}

// 自定义反序列化函数：将空字符串转换为 None
fn deserialize_optional_uuid<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => s
            .parse::<Uuid>()
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => Ok(Some(s)),
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Shared enum → string helpers (avoids duplication between registry_dispatch
// and handler.rs)
// ---------------------------------------------------------------------------

impl WebSocketCommand {
    /// Returns the string command_type for this variant.
    /// NOTE: `Subscribe`/`Unsubscribe` intentionally return their own names
    /// (matching the pre-Registry dispatch behavior).
    pub fn command_type(&self) -> &'static str {
        match self {
            WebSocketCommand::CreateLabel { .. } => "create_label",
            WebSocketCommand::UpdateLabel { .. } => "update_label",
            WebSocketCommand::DeleteLabel { .. } => "delete_label",
            WebSocketCommand::GetLabel { .. } => "get_label",
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
            WebSocketCommand::GetInvitation { .. } => "get_invitation",
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
            WebSocketCommand::GetProject { .. } => "get_project",
            WebSocketCommand::QueryProjects { .. } => "query_projects",
            WebSocketCommand::CreateIssue { .. } => "create_issue",
            WebSocketCommand::UpdateIssue { .. } => "update_issue",
            WebSocketCommand::DeleteIssue { .. } => "delete_issue",
            WebSocketCommand::QueryIssues { .. } => "query_issues",
            WebSocketCommand::GetIssue { .. } => "get_issue",
            WebSocketCommand::QueryCycles { .. } => "query_cycles",
            WebSocketCommand::GetCycle { .. } => "get_cycle",
            WebSocketCommand::CreateCycle { .. } => "create_cycle",
            WebSocketCommand::UpdateCycle { .. } => "update_cycle",
            WebSocketCommand::DeleteCycle { .. } => "delete_cycle",
            WebSocketCommand::GetFeatureFlags { .. } => "get_feature_flags",
        }
    }

    /// Extracts the request_id from this variant, if any.
    pub fn request_id(&self) -> Option<String> {
        let id = match self {
            WebSocketCommand::CreateLabel { request_id, .. }
            | WebSocketCommand::UpdateLabel { request_id, .. }
            | WebSocketCommand::DeleteLabel { request_id, .. }
            | WebSocketCommand::GetLabel { request_id, .. }
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
            | WebSocketCommand::GetInvitation { request_id, .. }
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
            | WebSocketCommand::GetProject { request_id, .. }
            | WebSocketCommand::QueryProjects { request_id, .. }
            | WebSocketCommand::CreateIssue { request_id, .. }
            | WebSocketCommand::UpdateIssue { request_id, .. }
            | WebSocketCommand::DeleteIssue { request_id, .. }
            | WebSocketCommand::QueryIssues { request_id, .. }
            | WebSocketCommand::GetIssue { request_id, .. }
            | WebSocketCommand::QueryCycles { request_id, .. }
            | WebSocketCommand::GetCycle { request_id, .. }
            | WebSocketCommand::CreateCycle { request_id, .. }
            | WebSocketCommand::UpdateCycle { request_id, .. }
            | WebSocketCommand::DeleteCycle { request_id, .. }
            | WebSocketCommand::GetFeatureFlags { request_id, .. } => request_id,
        };
        id.clone()
    }
}
