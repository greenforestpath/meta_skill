use insta::assert_snapshot;

use ms::error::MsError;

#[test]
fn test_error_skill_not_found() {
    let err = MsError::SkillNotFound("missing-skill".to_string());
    assert_snapshot!("error_skill_not_found", err.to_string());
}

#[test]
fn test_error_validation_failed() {
    let err = MsError::ValidationFailed("invalid input".to_string());
    assert_snapshot!("error_validation_failed", err.to_string());
}

#[test]
fn test_error_config_error() {
    let err = MsError::Config("missing config".to_string());
    assert_snapshot!("error_config_error", err.to_string());
}

#[test]
fn test_error_not_found() {
    let err = MsError::NotFound("resource missing".to_string());
    assert_snapshot!("error_not_found", err.to_string());
}
