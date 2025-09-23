#[test]
fn validate_create_project_status_rules() {
    use rust_backend::validation::project_status::validate_create_project_status;
    // ok name, no color
    assert!(validate_create_project_status("Planned", &None).is_ok());
    // empty name
    assert!(validate_create_project_status(" ", &None).is_err());
    // bad color
    assert!(validate_create_project_status("Active", &Some("red".to_string())).is_err());
    // good color
    assert!(validate_create_project_status("Active", &Some("#AABBCC".to_string())).is_ok());
}

#[test]
fn validate_update_project_status_rules() {
    use rust_backend::validation::project_status::{validate_update_project_status, UpdateProjectStatusChanges};
    use rust_backend::db::models::project_status::ProjectStatusCategory;

    // no fields
    let ch = UpdateProjectStatusChanges { name: None, description_present: false, color: None, category: None };
    assert!(validate_update_project_status(&ch).is_err());

    // empty name
    let ch = UpdateProjectStatusChanges { name: Some("  "), description_present: false, color: None, category: None };
    assert!(validate_update_project_status(&ch).is_err());

    // bad color
    let ch = UpdateProjectStatusChanges { name: None, description_present: false, color: Some("red"), category: None };
    assert!(validate_update_project_status(&ch).is_err());

    // valid combos
    let ch = UpdateProjectStatusChanges { name: Some("In Progress"), description_present: true, color: Some("#00FF00"), category: Some(ProjectStatusCategory::InProgress) };
    assert!(validate_update_project_status(&ch).is_ok());
}


