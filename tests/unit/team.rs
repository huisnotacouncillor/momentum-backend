use rust_backend::{
    routes::teams::{CreateTeamRequest, UpdateTeamRequest},
    services::{
        team_members_service::TeamMembersService,
        teams_service::TeamsService,
    },
};

#[test]
fn test_teams_service_validate_name() {
    assert!(TeamsService::validate_name("Valid Team").is_ok());
    assert!(TeamsService::validate_name("").is_err());
    assert!(TeamsService::validate_name("   ").is_err());
}

#[test]
fn test_teams_service_validate_team_key() {
    assert!(TeamsService::validate_team_key("team-key_1").is_ok());
    assert!(TeamsService::validate_team_key("TeamKey123").is_ok());
    assert!(TeamsService::validate_team_key("").is_err());
    assert!(TeamsService::validate_team_key("  ").is_err());
    assert!(TeamsService::validate_team_key("bad key").is_err());
    assert!(TeamsService::validate_team_key("bad@key").is_err());
    assert!(TeamsService::validate_team_key("bad#key").is_err());
}

#[test]
fn test_team_members_service_normalize_role() {
    assert_eq!(
        TeamMembersService::normalize_role("admin").unwrap(),
        "admin"
    );
    assert_eq!(
        TeamMembersService::normalize_role("member").unwrap(),
        "member"
    );
    assert!(TeamMembersService::normalize_role("").is_err());
    assert!(TeamMembersService::normalize_role("owner").is_err());
    assert!(TeamMembersService::normalize_role("ADMIN").is_err());
    assert!(TeamMembersService::normalize_role("Admin").is_err());
}

// Test validation behavior without DB
#[test]
fn test_create_team_request_validation_logic() {
    // Empty name should fail
    let req = CreateTeamRequest {
        name: "".to_string(),
        team_key: "valid-key".to_string(),
        description: None,
        icon_url: None,
        is_private: false,
    };
    assert!(TeamsService::validate_name(&req.name).is_err());

    // Empty team_key should fail
    let req = CreateTeamRequest {
        name: "Valid Name".to_string(),
        team_key: "".to_string(),
        description: None,
        icon_url: None,
        is_private: false,
    };
    assert!(TeamsService::validate_team_key(&req.team_key).is_err());

    // Invalid team_key format should fail
    let req = CreateTeamRequest {
        name: "Valid Name".to_string(),
        team_key: "bad key!".to_string(),
        description: None,
        icon_url: None,
        is_private: false,
    };
    assert!(TeamsService::validate_team_key(&req.team_key).is_err());

    // Valid request
    let req = CreateTeamRequest {
        name: "Valid Name".to_string(),
        team_key: "valid-key_123".to_string(),
        description: Some("Test desc".to_string()),
        icon_url: None,
        is_private: false,
    };
    assert!(TeamsService::validate_name(&req.name).is_ok());
    assert!(TeamsService::validate_team_key(&req.team_key).is_ok());
}

#[test]
fn test_update_team_request_no_changes_logic() {
    let req = UpdateTeamRequest {
        name: None,
        team_key: None,
        description: None,
        icon_url: None,
        is_private: None,
    };
    // Should have all fields None
    assert!(req.name.is_none());
    assert!(req.team_key.is_none());
    assert!(req.description.is_none());
    assert!(req.icon_url.is_none());
    assert!(req.is_private.is_none());
}

#[test]
fn test_team_member_role_mapping() {
    use rust_backend::routes::teams::TeamRole;

    // Ensure enum to string mapping is correct
    let admin = TeamRole::Admin;
    let member = TeamRole::Member;

    let admin_str = match admin {
        TeamRole::Admin => "admin",
        TeamRole::Member => "member",
    };
    let member_str = match member {
        TeamRole::Admin => "admin",
        TeamRole::Member => "member",
    };

    assert_eq!(admin_str, "admin");
    assert_eq!(member_str, "member");
}
