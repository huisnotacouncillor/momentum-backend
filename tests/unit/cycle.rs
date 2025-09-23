#[test]
fn validate_create_cycle_rules() {
    use rust_backend::validation::cycle::validate_create_cycle;
    assert!(validate_create_cycle("Sprint 1").is_ok());
    assert!(validate_create_cycle(" ").is_err());
}


