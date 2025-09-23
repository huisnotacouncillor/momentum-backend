// Validation-only tests for comments

#[test]
fn validate_comment_create_and_update() {
    use rust_backend::validation::comment::{validate_create_comment, validate_update_comment};
    assert!(validate_create_comment("hello").is_ok());
    assert!(validate_create_comment("").is_err());
    assert!(validate_create_comment(&"a".repeat(10001)).is_err());

    assert!(validate_update_comment("edit").is_ok());
    assert!(validate_update_comment(" ").is_err());
}


