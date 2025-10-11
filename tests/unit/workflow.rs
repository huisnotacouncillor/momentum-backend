#[test]
fn validate_workflow_and_state_rules() {
    use rust_backend::validation::workflow::{validate_create_state, validate_create_workflow};
    assert!(validate_create_workflow("Main").is_ok());
    assert!(validate_create_workflow(" ").is_err());

    assert!(validate_create_state("Todo", 0).is_ok());
    assert!(validate_create_state(" ", 0).is_err());
    assert!(validate_create_state("Todo", -1).is_err());
}
