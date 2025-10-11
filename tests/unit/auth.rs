// Unit tests focus on pure validation and DTO shaping

#[test]
fn validate_register_and_login_inputs() {
    use rust_backend::validation::auth::{
        UpdateProfileChanges, validate_login_request, validate_register_request,
        validate_update_profile,
    };

    // register
    assert!(
        validate_register_request("John Doe", "john_doe", "john@example.com", "StrongP4ss!")
            .is_ok()
    );
    assert!(validate_register_request("", "john", "john@example.com", "StrongP4ss!").is_err());
    assert!(validate_register_request("John", "jo", "john@example.com", "StrongP4ss!").is_err());
    assert!(validate_register_request("John", "john", "bad-email", "StrongP4ss!").is_err());
    assert!(validate_register_request("John", "john", "john@example.com", "weak").is_err());

    // login
    assert!(validate_login_request("john@example.com", "x").is_ok());
    assert!(validate_login_request("", "x").is_err());
    assert!(validate_login_request("john@example.com", "").is_err());

    // profile update
    let ok_changes = UpdateProfileChanges {
        name: Some("John"),
        username: None,
        email: None,
        avatar_url: None,
    };
    assert!(validate_update_profile(&ok_changes).is_ok());
    let empty_changes = UpdateProfileChanges {
        name: None,
        username: None,
        email: None,
        avatar_url: None,
    };
    assert!(validate_update_profile(&empty_changes).is_err());
}
