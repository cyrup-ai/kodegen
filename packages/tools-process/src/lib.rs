/// Platform-specific process ID type
///
/// This matches `tokio::process::Child::id()` return type and ensures
/// consistent PID handling across all process-related tools.
pub type ProcessId = u32;

pub mod list_processes;
pub use list_processes::*;

pub mod kill_process;
pub use kill_process::*;
