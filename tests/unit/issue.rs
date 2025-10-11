// Validation-only tests for issues

#[test]
fn validate_issue_create_and_update() {
    use rust_backend::validation::issue::{validate_create_issue, validate_update_issue};
    let team = uuid::Uuid::new_v4();

    assert!(validate_create_issue("Title", &None, &team).is_ok());
    assert!(validate_create_issue("", &None, &team).is_err());
    assert!(validate_create_issue(&"a".repeat(256), &None, &team).is_err());

    assert!(validate_update_issue(&Some("New".to_string()), &None).is_ok());
    assert!(validate_update_issue(&None, &Some("Desc".to_string())).is_ok());
    assert!(validate_update_issue(&None, &None).is_err());
}
