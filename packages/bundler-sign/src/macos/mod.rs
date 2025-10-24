//! macOS certificate provisioning for kodegen
//!
//! # Module Organization
//!
//! This module is decomposed into logical submodules:
//! - `validation` - Path validation, dependencies, and keychain checks
//! - `prompts` - User interaction and colored output macros
//! - `keychain` - Certificate and keychain operations
//! - `setup` - Main setup workflows
//! - `notarization` - Notarization workflow for macOS apps
//!
//! # Error Handling Strategy
//!
//! This module distinguishes between CRITICAL and DECORATIVE I/O operations:
//!
//! **CRITICAL I/O** - Errors propagated with `?` operator:
//!   • File operations: `fs::write()`, `fs::read_to_string()`, `fs::create_dir_all()`
//!   • User input: `io::stdin().read_line()`
//!   • External commands: `Command::new().output()`
//!   • Security operations: Permissions, keychain, certificate generation
//!
//!   These MUST succeed for the program to function correctly.
//!   Errors are propagated to the caller for proper handling.
//!
//! **DECORATIVE I/O** - Errors ignored with `let _ =`:
//!   • Terminal coloring: `buffer.set_color()`, writeln!(), `bufwtr.print()`
//!   • Status messages: Success/warning/error indicators with colors
//!
//!   These are nice-to-have but non-essential. If stderr is closed, TTY is
//!   detached, or output is redirected to a broken pipe, the program should
//!   continue without colors - not crash.
//!
//! This follows Rust CLI ecosystem best practices (cargo, rustc, ripgrep).
//!
//! Example:
//!   `io::stdout().flush()`?;              // Critical - propagate error
//!   let _ = `buffer.set_color`(...);      // Decorative - ignore error

pub mod validation;

#[macro_use]
pub mod prompts;

pub mod keychain;
pub mod notarization;
pub mod setup;

// Re-export public API
pub use keychain::{TempKeychain, sign_with_entitlements};
pub use notarization::{NotarizationAuth, diagnose_notarization_setup, notarize};
pub use setup::{ensure_api_key_file, interactive_setup, setup_from_config, show_config};
