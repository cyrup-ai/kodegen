//! Command validation and sanitization
//!
//! Provides comprehensive input validation with zero-allocation patterns and blazing-fast
//! validation algorithms for production-ready security and error handling.

// Submodule declarations
pub mod command_validators;
pub mod errors;
pub mod parameter_validators;
pub mod security;
pub mod validator;

// Re-export main types for backward compatibility
pub use errors::ValidationError;
pub use validator::CommandValidator;

use super::types::ImmutableChatCommand;
use std::sync::LazyLock;

/// Global validator instance
static GLOBAL_VALIDATOR: LazyLock<CommandValidator> = LazyLock::new(CommandValidator::new);

/// Get global validator
#[must_use]
pub fn get_global_validator() -> &'static CommandValidator {
    &GLOBAL_VALIDATOR
}

/// Validate command using global validator
///
/// # Errors
///
/// Returns `ValidationError` if command validation fails (see `CommandValidator::validate_command`)
pub fn validate_global_command(command: &ImmutableChatCommand) -> Result<(), ValidationError> {
    get_global_validator().validate_command(command)
}

/// Sanitize input using global validator
#[must_use]
pub fn sanitize_global_input(input: &str) -> String {
    get_global_validator().sanitize_input(input)
}

/// Check if input is safe using global validator
#[must_use]
pub fn is_global_safe_input(input: &str) -> bool {
    get_global_validator().is_safe_input(input)
}
