// Unit tests focus on pure validation; service tests would require DB setup

#[test]
fn validate_label_color_and_name() {
    use rust_backend::validation::label::validate_create_label;
    assert!(validate_create_label("bug", "#FF00AA").is_ok());
    assert!(validate_create_label(" ", "#FF00AA").is_err());
    assert!(validate_create_label("bug", "123456").is_err());
}

#[test]
fn validate_update_label_rules() {
    use rust_backend::validation::label::{UpdateLabelChanges, validate_update_label};

    // no fields -> error
    let c = UpdateLabelChanges {
        name: None,
        color: None,
        level_present: false,
    };
    assert!(validate_update_label(&c).is_err());

    // empty name -> error
    let c = UpdateLabelChanges {
        name: Some("  "),
        color: None,
        level_present: false,
    };
    assert!(validate_update_label(&c).is_err());

    // bad color -> error
    let c = UpdateLabelChanges {
        name: None,
        color: Some("red"),
        level_present: false,
    };
    assert!(validate_update_label(&c).is_err());

    // valid with name only
    let c = UpdateLabelChanges {
        name: Some("feature"),
        color: None,
        level_present: false,
    };
    assert!(validate_update_label(&c).is_ok());

    // valid with color only
    let c = UpdateLabelChanges {
        name: None,
        color: Some("#00FF00"),
        level_present: false,
    };
    assert!(validate_update_label(&c).is_ok());

    // valid with level only
    let c = UpdateLabelChanges {
        name: None,
        color: None,
        level_present: true,
    };
    assert!(validate_update_label(&c).is_ok());
}
