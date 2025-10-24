//! DEPRECATED: Shared Tokio runtime module
//!
//! This module is no longer needed since the application uses `#[tokio::main]`
//! which provides a runtime from the start. All code should use `tokio::spawn()`
//! directly instead of creating a separate runtime.
//!
//! This file is kept for backward compatibility but will be removed in a future version.

#[deprecated(
    since = "0.1.0",
    note = "Use tokio::spawn() directly instead. The main application provides a runtime via #[tokio::main]"
)]
pub fn shared_runtime() -> Option<&'static tokio::runtime::Runtime> {
    None
}
