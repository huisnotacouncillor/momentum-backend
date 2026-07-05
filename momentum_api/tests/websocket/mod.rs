//! WebSocket Integration Tests Module
//!
//! This module contains integration tests for WebSocket commands.
//! Tests require a running server - run with: cargo test -- --ignored

pub mod command_integration_tests;
pub mod fixtures;

pub use fixtures::{
    create_comment_command, create_team_command, create_team_workflow_status_command,
    delete_comment_command, delete_team_command, delete_team_workflow_status_command,
    delete_workspace_member_command, get_team_command, get_team_workflow_statuses_command,
    get_workspace_command, get_workspace_member_command, query_comments_command,
    update_comment_command, update_team_command, update_team_workflow_status_command,
    update_workspace_member_command, TestFixture, WebSocketTestConnection,
};
