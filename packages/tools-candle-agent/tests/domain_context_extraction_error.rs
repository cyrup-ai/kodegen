use kodegen_candle_agent::domain::context::ExtractionError;

#[test]
fn test_validation_error() {
    let err = ExtractionError::validation_failed("Invalid format");
    assert!(
        matches!(err, ExtractionError::ValidationFailed { reason } if reason == "Invalid format")
    );
}

#[test]
fn test_missing_fields_error() {
    let err = ExtractionError::missing_fields(&["name", "age"]);
    match err {
        ExtractionError::MissingFields { fields } => {
            assert_eq!(fields, vec!["name", "age"]);
        }
        _ => panic!("Expected MissingFields error"),
    }
}
