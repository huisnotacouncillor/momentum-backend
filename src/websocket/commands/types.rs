use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::db::enums::LabelLevel;

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
