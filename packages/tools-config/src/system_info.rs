use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system family ("macos", "linux", "windows", etc.)
    pub platform: String,

    /// CPU architecture ("`x86_64`", "`aarch64`", "`arm`", etc.)
    pub arch: String,

    /// OS version string (e.g., "macOS 14.6", "Ubuntu 22.04")
    pub os_version: String,

    /// Kernel version (e.g., "23.6.0" for macOS, "6.5.0-1" for Linux)
    pub kernel_version: String,

    /// Machine hostname
    pub hostname: String,

    /// Kodegen server version from Cargo.toml
    pub rust_version: String,

    /// Number of logical CPU cores
    pub cpu_count: usize,

    /// Memory information
    pub memory: MemoryInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_mb: String,
    pub available_mb: String,
    pub used_mb: String,
}

// Re-export rmcp's Implementation type as ClientInfo for API compatibility
pub use rmcp::model::Implementation as ClientInfo;

/// Client connection record with timestamp tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientRecord {
    pub client_info: ClientInfo,
    pub connected_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

/// Get current system information
///
/// Collects cross-platform diagnostic data using the sysinfo crate.
/// All fields are guaranteed to be populated (using fallbacks if collection fails).
#[must_use]
pub fn get_system_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Memory information (sysinfo returns kilobytes)
    let total_kb = sys.total_memory();
    let available_kb = sys.available_memory();
    let used_kb = sys.used_memory();

    SystemInfo {
        // Platform from std::env (always available)
        platform: std::env::consts::OS.to_string(),

        // Architecture from std::env (always available)
        arch: std::env::consts::ARCH.to_string(),

        // OS version with fallback
        os_version: System::long_os_version()
            .unwrap_or_else(|| format!("{} (unknown version)", std::env::consts::OS)),

        // Kernel version with fallback
        kernel_version: System::kernel_version().unwrap_or_else(|| "unknown".to_string()),

        // Hostname with fallback
        hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),

        // Server version from build-time environment variable
        rust_version: env!("CARGO_PKG_VERSION").to_string(),

        // CPU count (number of logical cores)
        cpu_count: sys.cpus().len(),

        // Memory info converted to MB for readability
        memory: MemoryInfo {
            total_mb: format!("{} MB", total_kb / 1024),
            available_mb: format!("{} MB", available_kb / 1024),
            used_mb: format!("{} MB", used_kb / 1024),
        },
    }
}
