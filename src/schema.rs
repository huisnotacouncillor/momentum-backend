// @generated automatically by Diesel CLI.

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
        team_id -> Uuid,
        project_id -> Nullable<Uuid>,
        cycle_id -> Nullable<Uuid>,
        creator_id -> Uuid,
        assignee_id -> Nullable<Uuid>,
        parent_issue_id -> Nullable<Uuid>,
        issue_number -> Int4,
        #[max_length = 512]
        title -> Varchar,
        description -> Nullable<Text>,
        status -> Text,
        priority -> Text,
        is_changelog_candidate -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    labels (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 7]
        color -> Nullable<Varchar>,
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
        status -> Text,
        target_date -> Nullable<Date>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
diesel::joinable!(issue_labels -> issues (issue_id));
diesel::joinable!(issue_labels -> labels (label_id));
diesel::joinable!(issues -> cycles (cycle_id));
diesel::joinable!(issues -> projects (project_id));
diesel::joinable!(issues -> teams (team_id));
diesel::joinable!(labels -> workspaces (workspace_id));
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

diesel::allow_tables_to_appear_in_same_query!(
    comments,
    cycles,
    issue_labels,
    issues,
    labels,
    oauth_providers,
    projects,
    roadmaps,
    team_members,
    teams,
    user_credentials,
    user_sessions,
    users,
    workspaces,
);
