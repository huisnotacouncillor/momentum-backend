#[test]
fn validate_create_project_rules() {
    use rust_backend::validation::project::validate_create_project;
    assert!(validate_create_project("Alpha", "ALPHA").is_ok());
    assert!(validate_create_project(" ", "ALPHA").is_err());
    assert!(validate_create_project("Alpha", "").is_err());
    assert!(validate_create_project("Alpha", "TOO_LONG_KEY_123").is_err());
    assert!(validate_create_project("Alpha", "BAD KEY").is_err());
}
