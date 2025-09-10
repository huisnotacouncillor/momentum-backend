// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "invitation_status"))]
    pub struct InvitationStatus;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "label_level_enum"))]
    pub struct LabelLevelEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "workspace_user_role"))]
    pub struct WorkspaceUserRole;
}

diesel::table! {
    comments (id) {
        id -> Uuid,
        issue_id -> Uuid,
        author_id -> Uuid,
        body -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    cycles (id) {
        id -> Uuid,
        team_id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        start_date -> Date,
        end_date -> Date,
        status -> Text,
        created_at -> Timestamptz,
        description -> Nullable<Text>,
        goal -> Nullable<Text>,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::WorkspaceUserRole;
    use super::sql_types::InvitationStatus;

    invitations (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        #[max_length = 255]
        email -> Varchar,
        role -> WorkspaceUserRole,
        status -> InvitationStatus,
        invited_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        expires_at -> Timestamptz,
    }
}

diesel::table! {
    issue_labels (issue_id, label_id) {
        issue_id -> Uuid,
        label_id -> Uuid,
    }
}

diesel::table! {
    issues (id) {
        id -> Uuid,
        project_id -> Nullable<Uuid>,
        cycle_id -> Nullable<Uuid>,
        creator_id -> Uuid,
        assignee_id -> Nullable<Uuid>,
        parent_issue_id -> Nullable<Uuid>,
        issue_number -> Int4,
        #[max_length = 512]
        title -> Varchar,
        description -> Nullable<Text>,
        priority -> Text,
        is_changelog_candidate -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        team_id -> Uuid,
        workflow_id -> Nullable<Uuid>,
        workflow_state_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::LabelLevelEnum;

    labels (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 7]
        color -> Varchar,
        level -> LabelLevelEnum,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    oauth_providers (id) {
        id -> Int4,
        #[max_length = 100]
        provider_name -> Varchar,
        #[max_length = 255]
        client_id -> Varchar,
        #[max_length = 255]
        client_secret -> Varchar,
        auth_url -> Text,
        token_url -> Text,
        user_info_url -> Text,
        #[max_length = 255]
        scope -> Nullable<Varchar>,
        is_active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    project_statuses (id) {
        id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        #[max_length = 7]
        color -> Nullable<Varchar>,
        #[max_length = 50]
        category -> Varchar,
        workspace_id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    projects (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        roadmap_id -> Nullable<Uuid>,
        owner_id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 10]
        project_key -> Varchar,
        description -> Nullable<Text>,
        target_date -> Nullable<Date>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        project_status_id -> Uuid,
        priority -> Text,
    }
}

diesel::table! {
    roadmaps (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        start_date -> Date,
        end_date -> Date,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    team_members (user_id, team_id) {
        user_id -> Uuid,
        team_id -> Uuid,
        #[max_length = 50]
        role -> Varchar,
        joined_at -> Timestamptz,
    }
}

diesel::table! {
    teams (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 10]
        team_key -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        description -> Nullable<Text>,
        icon_url -> Nullable<Text>,
        is_private -> Bool,
    }
}

diesel::table! {
    user_credentials (id) {
        id -> Int4,
        user_id -> Uuid,
        #[max_length = 50]
        credential_type -> Varchar,
        credential_hash -> Nullable<Text>,
        #[max_length = 100]
        oauth_provider_id -> Nullable<Varchar>,
        #[max_length = 255]
        oauth_user_id -> Nullable<Varchar>,
        is_primary -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    user_sessions (id) {
        id -> Int4,
        user_id -> Uuid,
        #[max_length = 255]
        session_token -> Varchar,
        #[max_length = 255]
        refresh_token -> Nullable<Varchar>,
        device_info -> Nullable<Text>,
        ip_address -> Nullable<Inet>,
        user_agent -> Nullable<Text>,
        expires_at -> Timestamp,
        is_active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        name -> Text,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 100]
        username -> Varchar,
        avatar_url -> Nullable<Text>,
        is_active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        id -> Uuid,
        current_workspace_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    workflow_states (id) {
        id -> Uuid,
        workflow_id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        #[max_length = 7]
        color -> Nullable<Varchar>,
        #[max_length = 50]
        category -> Varchar,
        position -> Int4,
        is_default -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    workflow_transitions (id) {
        id -> Uuid,
        workflow_id -> Uuid,
        from_state_id -> Nullable<Uuid>,
        to_state_id -> Uuid,
        #[max_length = 255]
        name -> Nullable<Varchar>,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    workflows (id) {
        id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        team_id -> Uuid,
        is_default -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::WorkspaceUserRole;

    workspace_members (user_id, workspace_id) {
        user_id -> Uuid,
        workspace_id -> Uuid,
        role -> WorkspaceUserRole,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    workspaces (id) {
        id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        url_key -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(comments -> issues (issue_id));
diesel::joinable!(comments -> users (author_id));
diesel::joinable!(cycles -> teams (team_id));
diesel::joinable!(invitations -> users (invited_by));
diesel::joinable!(invitations -> workspaces (workspace_id));
diesel::joinable!(issue_labels -> issues (issue_id));
diesel::joinable!(issue_labels -> labels (label_id));
diesel::joinable!(issues -> cycles (cycle_id));
diesel::joinable!(issues -> projects (project_id));
diesel::joinable!(issues -> teams (team_id));
diesel::joinable!(issues -> workflow_states (workflow_state_id));
diesel::joinable!(issues -> workflows (workflow_id));
diesel::joinable!(labels -> workspaces (workspace_id));
diesel::joinable!(project_statuses -> workspaces (workspace_id));
diesel::joinable!(projects -> project_statuses (project_status_id));
diesel::joinable!(projects -> roadmaps (roadmap_id));
diesel::joinable!(projects -> users (owner_id));
diesel::joinable!(projects -> workspaces (workspace_id));
diesel::joinable!(roadmaps -> workspaces (workspace_id));
diesel::joinable!(team_members -> teams (team_id));
diesel::joinable!(team_members -> users (user_id));
diesel::joinable!(teams -> workspaces (workspace_id));
diesel::joinable!(user_credentials -> users (user_id));
diesel::joinable!(user_sessions -> users (user_id));
diesel::joinable!(users -> workspaces (current_workspace_id));
diesel::joinable!(workflow_states -> workflows (workflow_id));
diesel::joinable!(workflow_transitions -> workflows (workflow_id));
diesel::joinable!(workflows -> teams (team_id));
diesel::joinable!(workspace_members -> users (user_id));
diesel::joinable!(workspace_members -> workspaces (workspace_id));

diesel::allow_tables_to_appear_in_same_query!(
    comments,
    cycles,
    invitations,
    issue_labels,
    issues,
    labels,
    oauth_providers,
    project_statuses,
    projects,
    roadmaps,
    team_members,
    teams,
    user_credentials,
    user_sessions,
    users,
    workflow_states,
    workflow_transitions,
    workflows,
    workspace_members,
    workspaces,
);
