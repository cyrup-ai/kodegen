//! Error conversion traits and Result extensions

use super::core::{ZeroAllocError, ZeroAllocResult};
use super::stats::record_error;
use super::types::{ErrorCategory, ErrorRecoverability, ErrorSeverity};

/// Trait for converting errors to zero-allocation errors
pub trait IntoZeroAllocError {
    fn into_zero_alloc_error(self) -> ZeroAllocError;
}

impl IntoZeroAllocError for std::io::Error {
    fn into_zero_alloc_error(self) -> ZeroAllocError {
        ZeroAllocError::new(
            ErrorCategory::System,
            ErrorSeverity::Error,
            ErrorRecoverability::Retriable,
            &self.to_string(),
            u64::from(self.raw_os_error().unwrap_or(0).unsigned_abs()),
        )
    }
}

impl IntoZeroAllocError for serde_json::Error {
    fn into_zero_alloc_error(self) -> ZeroAllocError {
        ZeroAllocError::new(
            ErrorCategory::Serialization,
            ErrorSeverity::Error,
            ErrorRecoverability::Permanent,
            &self.to_string(),
            self.line() as u64,
        )
    }
}

impl<T> IntoZeroAllocError for std::sync::PoisonError<T> {
    fn into_zero_alloc_error(self) -> ZeroAllocError {
        ZeroAllocError::new(
            ErrorCategory::System,
            ErrorSeverity::Critical,
            ErrorRecoverability::Manual,
            "Mutex poison error",
            1001,
        )
    }
}

impl IntoZeroAllocError for std::num::ParseIntError {
    fn into_zero_alloc_error(self) -> ZeroAllocError {
        ZeroAllocError::new(
            ErrorCategory::User,
            ErrorSeverity::Error,
            ErrorRecoverability::Permanent,
            &self.to_string(),
            1002,
        )
    }
}

impl IntoZeroAllocError for std::num::ParseFloatError {
    fn into_zero_alloc_error(self) -> ZeroAllocError {
        ZeroAllocError::new(
            ErrorCategory::User,
            ErrorSeverity::Error,
            ErrorRecoverability::Permanent,
            &self.to_string(),
            1003,
        )
    }
}

impl IntoZeroAllocError for std::str::Utf8Error {
    fn into_zero_alloc_error(self) -> ZeroAllocError {
        ZeroAllocError::new(
            ErrorCategory::Serialization,
            ErrorSeverity::Error,
            ErrorRecoverability::Permanent,
            &self.to_string(),
            1004,
        )
    }
}

impl IntoZeroAllocError for std::time::SystemTimeError {
    fn into_zero_alloc_error(self) -> ZeroAllocError {
        ZeroAllocError::new(
            ErrorCategory::System,
            ErrorSeverity::Error,
            ErrorRecoverability::Retriable,
            &self.to_string(),
            1005,
        )
    }
}

/// Extension trait for Result types
pub trait ZeroAllocResultExt<T> {
    /// Map error to `ZeroAllocError`
    ///
    /// # Errors
    ///
    /// Returns `ZeroAllocError` from mapper function if self is Err
    fn map_zero_alloc_err<F>(self, f: F) -> Result<T, Box<ZeroAllocError>>
    where
        F: FnOnce() -> ZeroAllocError;

    /// Add metadata to error
    ///
    /// # Errors
    ///
    /// Returns original error with added metadata if self is Err
    fn with_error_metadata(self, key: &str, value: &str) -> Result<T, Box<ZeroAllocError>>;

    /// Add error code
    ///
    /// # Errors
    ///
    /// Returns original error with added code if self is Err
    fn with_error_code(self, code: u64) -> Result<T, Box<ZeroAllocError>>;

    /// Record error to global counter
    ///
    /// # Errors
    ///
    /// Returns original error after recording if self is Err
    fn record_error(self) -> Result<T, Box<ZeroAllocError>>;
}

impl<T, E> ZeroAllocResultExt<T> for Result<T, E>
where
    E: IntoZeroAllocError,
{
    fn map_zero_alloc_err<F>(self, f: F) -> Result<T, Box<ZeroAllocError>>
    where
        F: FnOnce() -> ZeroAllocError,
    {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(Box::new(f())),
        }
    }

    fn with_error_metadata(self, key: &str, value: &str) -> Result<T, Box<ZeroAllocError>> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => Err(Box::new(
                e.into_zero_alloc_error().with_metadata(key, value),
            )),
        }
    }

    fn with_error_code(self, code: u64) -> Result<T, Box<ZeroAllocError>> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => {
                let mut error = e.into_zero_alloc_error();
                error.code = code;
                Err(Box::new(error))
            }
        }
    }

    fn record_error(self) -> Result<T, Box<ZeroAllocError>> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => {
                let error = e.into_zero_alloc_error();
                record_error(&error);
                Err(Box::new(error))
            }
        }
    }
}

impl<T> ZeroAllocResultExt<T> for ZeroAllocResult<T> {
    fn map_zero_alloc_err<F>(self, f: F) -> Result<T, Box<ZeroAllocError>>
    where
        F: FnOnce() -> ZeroAllocError,
    {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(Box::new(f())),
        }
    }

    fn with_error_metadata(self, key: &str, value: &str) -> Result<T, Box<ZeroAllocError>> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => Err(Box::new(e.with_metadata(key, value))),
        }
    }

    fn with_error_code(self, code: u64) -> Result<T, Box<ZeroAllocError>> {
        match self {
            Ok(value) => Ok(value),
            Err(mut e) => {
                e.code = code;
                Err(Box::new(e))
            }
        }
    }

    fn record_error(self) -> Result<T, Box<ZeroAllocError>> {
        match self {
            Ok(value) => Ok(value),
            Err(e) => {
                record_error(&e);
                Err(Box::new(e))
            }
        }
    }
}
