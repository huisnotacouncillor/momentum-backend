#[test]
fn placeholder_role_change_validation() {
    use rust_backend::validation::workspace_member::validate_role_change;
    assert!(validate_role_change().is_ok());
}
