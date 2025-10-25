//! Error types for the model system

use std::borrow::Cow;
use std::error::Error;
use std::fmt;

/// Error type for Candle model operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandleModelError {
    /// Model not found in registry
    ModelNotFound {
        provider: Cow<'static, str>,
        name: Cow<'static, str>,
    },

    /// Provider not found in registry
    ProviderNotFound(Cow<'static, str>),

    /// Model already exists in registry
    ModelAlreadyExists {
        provider: Cow<'static, str>,
        name: Cow<'static, str>,
    },

    /// Invalid model configuration
    InvalidConfiguration(Cow<'static, str>),

    /// Operation not supported by model
    OperationNotSupported(Cow<'static, str>),

    /// Invalid input data
    InvalidInput(Cow<'static, str>),

    /// Internal error (should be used sparingly)
    Internal(Cow<'static, str>),

    /// Type mismatch when accessing registered model
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
}

impl fmt::Display for CandleModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelNotFound { provider, name } => {
                write!(f, "Model not found: {provider}:{name}")
            }
            Self::ProviderNotFound(provider) => write!(f, "Provider not found: {provider}"),
            Self::ModelAlreadyExists { provider, name } => {
                write!(f, "Model already registered: {provider}:{name}")
            }
            Self::InvalidConfiguration(msg) => write!(f, "Invalid model configuration: {msg}"),
            Self::OperationNotSupported(msg) => {
                write!(f, "Operation not supported by model: {msg}")
            }
            Self::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),
            Self::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {expected}, found {found}")
            }
        }
    }
}

impl Error for CandleModelError {}

impl From<candle_core::Error> for CandleModelError {
    fn from(err: candle_core::Error) -> Self {
        Self::Internal(err.to_string().into())
    }
}

/// Result type for Candle model operations
pub type CandleResult<T> = std::result::Result<T, CandleModelError>;

/// Extension trait for converting Option to `ModelError`
pub trait OptionExt<T> {
    /// Convert an Option to a Result with a `CandleModelError::ModelNotFound`
    ///
    /// # Errors
    ///
    /// Returns `CandleModelError::ModelNotFound` if the Option is None
    fn or_model_not_found<P, N>(self, provider: P, name: N) -> CandleResult<T>
    where
        P: Into<Cow<'static, str>>,
        N: Into<Cow<'static, str>>;
}

impl<T> OptionExt<T> for Option<T> {
    fn or_model_not_found<P, N>(self, provider: P, name: N) -> CandleResult<T>
    where
        P: Into<Cow<'static, str>>,
        N: Into<Cow<'static, str>>,
    {
        self.ok_or_else(|| CandleModelError::ModelNotFound {
            provider: provider.into(),
            name: name.into(),
        })
    }
}

/// Extension trait for converting Result to `ModelError`
pub trait ResultExt<T, E> {
    /// Map an error to a `CandleModelError::InvalidConfiguration`
    ///
    /// # Errors
    ///
    /// Returns `CandleModelError::InvalidConfiguration` if the Result is Err
    fn invalid_config<M>(self, msg: M) -> CandleResult<T>
    where
        M: Into<Cow<'static, str>>;

    /// Map an error to a `CandleModelError::OperationNotSupported`
    ///
    /// # Errors
    ///
    /// Returns `CandleModelError::OperationNotSupported` if the Result is Err
    fn not_supported<M>(self, msg: M) -> CandleResult<T>
    where
        M: Into<Cow<'static, str>>;
}

impl<T, E: Error> ResultExt<T, E> for std::result::Result<T, E> {
    fn invalid_config<M>(self, msg: M) -> CandleResult<T>
    where
        M: Into<Cow<'static, str>>,
    {
        self.map_err(|_| CandleModelError::InvalidConfiguration(msg.into()))
    }

    fn not_supported<M>(self, msg: M) -> CandleResult<T>
    where
        M: Into<Cow<'static, str>>,
    {
        self.map_err(|_| CandleModelError::OperationNotSupported(msg.into()))
    }
}

/// Helper for creating error messages with static strings
#[macro_export]
macro_rules! model_err {
    (not_found: $provider:expr, $name:expr) => {
        $crate::domain::model::error::CandleModelError::ModelNotFound {
            provider: $provider.into(),
            name: $name.into(),
        }
    };
    (provider_not_found: $provider:expr) => {
        $crate::domain::model::error::CandleModelError::ProviderNotFound($provider.into())
    };
    (already_exists: $provider:expr, $name:expr) => {
        $crate::domain::model::error::CandleModelError::ModelAlreadyExists {
            provider: $provider.into(),
            name: $name.into(),
        }
    };
    (invalid_config: $msg:expr) => {
        $crate::domain::model::error::CandleModelError::InvalidConfiguration($msg.into())
    };
    (not_supported: $msg:expr) => {
        $crate::domain::model::error::CandleModelError::OperationNotSupported($msg.into())
    };
    (invalid_input: $msg:expr) => {
        $crate::domain::model::error::CandleModelError::InvalidInput($msg.into())
    };
    (internal: $msg:expr) => {
        $crate::domain::model::error::CandleModelError::Internal($msg.into())
    };
}

/// Helper for creating error results
#[macro_export]
macro_rules! bail_model_err {
    ($($tokens:tt)*) => {
        return Err(model_err!($($tokens)*))
    };
}
