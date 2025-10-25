// Tests extracted from src/domain/model/error.rs

use kodegen_candle_agent::domain::model::error::{
    CandleModelError, OptionExt, ResultExt,
};
use kodegen_candle_agent::model_err;
use std::borrow::Cow;
use std::fmt;

#[test]
fn test_model_error_display() {
    assert_eq!(
        CandleModelError::ModelNotFound {
            provider: Cow::Borrowed("test"),
            name: Cow::Borrowed("test")
        }
        .to_string(),
        "Model not found: test:test"
    );
    assert_eq!(
        CandleModelError::ProviderNotFound(Cow::Borrowed("test")).to_string(),
        "Provider not found: test"
    );
    assert_eq!(
        CandleModelError::ModelAlreadyExists {
            provider: Cow::Borrowed("test"),
            name: Cow::Borrowed("test")
        }
        .to_string(),
        "Model already registered: test:test"
    );
    assert_eq!(
        CandleModelError::InvalidConfiguration(Cow::Borrowed("test")).to_string(),
        "Invalid model configuration: test"
    );
    assert_eq!(
        CandleModelError::OperationNotSupported(Cow::Borrowed("test")).to_string(),
        "Operation not supported by model: test"
    );
    assert_eq!(
        CandleModelError::InvalidInput(Cow::Borrowed("test")).to_string(),
        "Invalid input: test"
    );
    assert_eq!(
        CandleModelError::Internal(Cow::Borrowed("test")).to_string(),
        "Internal error: test"
    );
}

#[test]
fn test_option_ext() -> Result<(), Box<dyn std::error::Error>> {
    let some: Option<u32> = Some(42);
    assert_eq!(some.or_model_not_found("test", "test")?, 42);

    let none: Option<u32> = None;
    assert!(matches!(
        none.or_model_not_found("test", "test"),
        Err(CandleModelError::ModelNotFound {
            provider: _,
            name: _
        })
    ));
    Ok(())
}

#[test]
fn test_result_ext() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Debug, Clone)]
    struct TestError(String);

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    let ok: std::result::Result<u32, TestError> = Ok(42);
    assert_eq!(ok.clone().invalid_config("test")?, 42);
    assert_eq!(ok.not_supported("test")?, 42);

    let err: std::result::Result<u32, TestError> = Err(TestError("error".to_string()));
    assert!(matches!(
        err.clone().invalid_config("test"),
        Err(CandleModelError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        err.not_supported("test"),
        Err(CandleModelError::OperationNotSupported(_))
    ));
    Ok(())
}

#[test]
fn test_model_err_macro() {
    assert!(matches!(
        model_err!(not_found: "test", "test"),
        CandleModelError::ModelNotFound {
            provider: _,
            name: _
        }
    ));
    assert!(matches!(
        model_err!(provider_not_found: "test"),
        CandleModelError::ProviderNotFound(_)
    ));
    assert!(matches!(
        model_err!(already_exists: "test", "test"),
        CandleModelError::ModelAlreadyExists {
            provider: _,
            name: _
        }
    ));
    assert!(matches!(
        model_err!(invalid_config: "test"),
        CandleModelError::InvalidConfiguration(_)
    ));
    assert!(matches!(
        model_err!(not_supported: "test"),
        CandleModelError::OperationNotSupported(_)
    ));
    assert!(matches!(
        model_err!(invalid_input: "test"),
        CandleModelError::InvalidInput(_)
    ));
    assert!(matches!(
        model_err!(internal: "test"),
        CandleModelError::Internal(_)
    ));
}
