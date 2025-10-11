#[test]
fn validate_create_workspace_rules() {
    use rust_backend::validation::workspace::validate_create_workspace;
    assert!(validate_create_workspace("Acme", "acme").is_ok());
    assert!(validate_create_workspace(" ", "acme").is_err());
    assert!(validate_create_workspace("Acme", "").is_err());
    assert!(validate_create_workspace("Acme", "Acme").is_err());
    assert!(validate_create_workspace("Acme", "-acme").is_err());
    assert!(validate_create_workspace("Acme", "acme-").is_err());
}
