#[test]
fn validate_invite_email_rules() {
    use rust_backend::validation::invitation::validate_invite_email;
    assert!(validate_invite_email("user@example.com").is_ok());
    assert!(validate_invite_email("").is_err());
    assert!(validate_invite_email("no-at.com").is_err());
}


