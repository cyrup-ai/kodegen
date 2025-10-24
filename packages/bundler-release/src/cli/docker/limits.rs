//! Resource limits for Docker containers.
//!
//! Controls memory, CPU, and process limits to prevent containers from
//! consuming excessive host resources during cross-platform builds.

use sysinfo::System;

/// Resource limits for Docker containers.
///
/// Controls memory, CPU, and process limits to prevent containers from
/// consuming excessive host resources during cross-platform builds.
#[derive(Debug, Clone)]
pub struct ContainerLimits {
    /// Maximum memory (e.g., "4g", "2048m")
    pub memory: String,

    /// Maximum memory + swap (e.g., "6g", "3072m")
    pub memory_swap: String,

    /// Number of CPUs (fractional allowed, e.g., "2", "1.5")
    pub cpus: String,

    /// Maximum number of processes
    pub pids_limit: u32,
}

impl Default for ContainerLimits {
    fn default() -> Self {
        Self::detect_safe_limits()
    }
}

impl ContainerLimits {
    /// Detects safe resource limits based on host system capabilities.
    ///
    /// Uses conservative defaults:
    /// - Memory: 50% of total RAM (minimum 2GB, maximum 8GB)
    /// - Swap: Memory + 2GB
    /// - CPUs: 50% of available cores (minimum 2)
    /// - PIDs: 1000 (sufficient for most builds, prevents fork bombs)
    pub fn detect_safe_limits() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();

        // Calculate memory limit (50% of total, min 2GB, max 8GB)
        let total_ram_gb = sys.total_memory() / 1024 / 1024 / 1024;
        let memory_gb = (total_ram_gb / 2).clamp(2, 8);
        let swap_gb = memory_gb + 2;

        // Calculate CPU limit (50% of cores, minimum 2)
        let total_cpus = num_cpus::get();
        let cpu_limit = (total_cpus / 2).max(2);

        Self {
            memory: format!("{}g", memory_gb),
            memory_swap: format!("{}g", swap_gb),
            cpus: cpu_limit.to_string(),
            pids_limit: 1000,
        }
    }

    /// Parse memory string like "4g", "4096m", "4G", "2048M" to megabytes.
    fn parse_memory_to_mb(memory: &str) -> Result<u64, String> {
        let memory = memory.trim().to_lowercase();

        if let Some(stripped) = memory.strip_suffix("gb") {
            let val: u64 = stripped
                .parse()
                .map_err(|_| format!("Invalid memory value: {}", memory))?;
            Ok(val * 1024)
        } else if let Some(stripped) = memory.strip_suffix("g") {
            let val: u64 = stripped
                .parse()
                .map_err(|_| format!("Invalid memory value: {}", memory))?;
            Ok(val * 1024)
        } else if let Some(stripped) = memory.strip_suffix("mb") {
            stripped
                .parse()
                .map_err(|_| format!("Invalid memory value: {}", memory))
        } else if let Some(stripped) = memory.strip_suffix("m") {
            stripped
                .parse()
                .map_err(|_| format!("Invalid memory value: {}", memory))
        } else {
            // No unit - assume megabytes
            memory
                .parse()
                .map_err(|_| format!("Invalid memory value: {}", memory))
        }
    }

    /// Creates limits from CLI arguments.
    ///
    /// Validates that memory_swap >= memory.
    pub fn from_cli(
        memory: String,
        memory_swap: Option<String>,
        cpus: Option<String>,
        pids_limit: u32,
    ) -> Result<Self, String> {
        let memory_mb = Self::parse_memory_to_mb(&memory)?;

        // Validate minimum (Docker requires 4MB, we require 512MB for builds)
        if memory_mb < 512 {
            return Err(format!(
                "Memory limit too low: {} MB (minimum: 512 MB)\n\
                 Docker builds require significant memory for compilation.",
                memory_mb
            ));
        }

        // Validate maximum (sanity check: 1TB)
        if memory_mb > 1024 * 1024 {
            return Err(format!(
                "Memory limit too high: {} MB (maximum: 1 TB)",
                memory_mb
            ));
        }

        let memory_swap = if let Some(swap) = memory_swap {
            let swap_mb = Self::parse_memory_to_mb(&swap)?;

            // Validate: swap must be >= memory
            if swap_mb < memory_mb {
                return Err(format!(
                    "Memory swap ({} MB) must be >= memory ({} MB)",
                    swap_mb, memory_mb
                ));
            }

            format!("{}m", swap_mb)
        } else {
            // Default: memory + 2GB
            format!("{}m", memory_mb + 2048)
        };

        // Validate CPUs
        let cpus = if let Some(cpus_str) = cpus {
            let cpus_f32: f32 = cpus_str.parse().map_err(|_| {
                format!(
                    "Invalid --cpus value: '{}' (expected number like '2' or '1.5')",
                    cpus_str
                )
            })?;

            if cpus_f32 <= 0.0 {
                return Err(format!("CPU limit must be positive, got: {}", cpus_f32));
            }

            if cpus_f32 > 1024.0 {
                return Err(format!("CPU limit too high: {} (maximum: 1024)", cpus_f32));
            }

            cpus_str
        } else {
            num_cpus::get().to_string()
        };

        // Validate PID limit
        if pids_limit < 10 {
            return Err(format!(
                "PID limit too low: {} (minimum: 10)\n\
                 Builds require multiple processes.",
                pids_limit
            ));
        }

        if pids_limit > 1_000_000 {
            return Err(format!(
                "PID limit too high: {} (maximum: 1,000,000)",
                pids_limit
            ));
        }

        Ok(Self {
            memory, // Keep original format for Docker
            memory_swap,
            cpus,
            pids_limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memory_lowercase_g() {
        assert_eq!(ContainerLimits::parse_memory_to_mb("4g"), Ok(4096));
    }

    #[test]
    fn test_parse_memory_uppercase_g() {
        assert_eq!(ContainerLimits::parse_memory_to_mb("4G"), Ok(4096));
    }

    #[test]
    fn test_parse_memory_lowercase_gb() {
        assert_eq!(ContainerLimits::parse_memory_to_mb("4gb"), Ok(4096));
    }

    #[test]
    fn test_parse_memory_uppercase_gb() {
        assert_eq!(ContainerLimits::parse_memory_to_mb("4GB"), Ok(4096));
    }

    #[test]
    fn test_parse_memory_lowercase_m() {
        assert_eq!(ContainerLimits::parse_memory_to_mb("4096m"), Ok(4096));
    }

    #[test]
    fn test_parse_memory_uppercase_m() {
        assert_eq!(ContainerLimits::parse_memory_to_mb("4096M"), Ok(4096));
    }

    #[test]
    fn test_parse_memory_lowercase_mb() {
        assert_eq!(ContainerLimits::parse_memory_to_mb("4096mb"), Ok(4096));
    }

    #[test]
    fn test_parse_memory_uppercase_mb() {
        assert_eq!(ContainerLimits::parse_memory_to_mb("4096MB"), Ok(4096));
    }

    #[test]
    fn test_parse_memory_no_unit() {
        // No unit = assume megabytes
        assert_eq!(ContainerLimits::parse_memory_to_mb("2048"), Ok(2048));
    }

    #[test]
    fn test_parse_memory_invalid_text() {
        assert!(ContainerLimits::parse_memory_to_mb("invalid").is_err());
    }

    #[test]
    fn test_parse_memory_invalid_unit() {
        assert!(ContainerLimits::parse_memory_to_mb("4x").is_err());
    }

    #[test]
    fn test_parse_memory_with_spaces() {
        assert_eq!(ContainerLimits::parse_memory_to_mb("  4g  "), Ok(4096));
    }

    #[test]
    fn test_from_cli_default_swap() {
        let result = ContainerLimits::from_cli("4g".to_string(), None, None, 1000);
        assert!(result.is_ok());
        let limits = result.unwrap();
        // 4GB + 2GB = 6GB = 6144MB
        assert_eq!(limits.memory_swap, "6144m");
    }

    #[test]
    fn test_from_cli_correct_unit_conversion_for_megabytes() {
        // This is the critical test case from the bug report
        // "4096m" should be treated as 4GB, so default swap = 4GB + 2GB = 6GB = 6144MB
        let result = ContainerLimits::from_cli("4096m".to_string(), None, None, 1000);
        assert!(result.is_ok());
        let limits = result.unwrap();
        assert_eq!(limits.memory_swap, "6144m");
        // NOT "4098g" as the bug would have produced!
    }

    #[test]
    fn test_from_cli_swap_validation_success() {
        // Swap >= memory should succeed
        let result =
            ContainerLimits::from_cli("4g".to_string(), Some("6g".to_string()), None, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_cli_swap_validation_failure() {
        // Swap < memory should fail
        let result =
            ContainerLimits::from_cli("8g".to_string(), Some("4g".to_string()), None, 1000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("must be >="));
    }

    #[test]
    fn test_from_cli_swap_equal_to_memory() {
        // Swap == memory should succeed
        let result =
            ContainerLimits::from_cli("4g".to_string(), Some("4g".to_string()), None, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_cli_preserves_original_memory_format() {
        // Should keep original memory format for Docker
        let result =
            ContainerLimits::from_cli("4g".to_string(), Some("6g".to_string()), None, 1000);
        assert!(result.is_ok());
        let limits = result.unwrap();
        assert_eq!(limits.memory, "4g");
    }

    #[test]
    fn test_from_cli_invalid_memory_format() {
        let result = ContainerLimits::from_cli("invalid".to_string(), None, None, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_cli_invalid_swap_format() {
        let result =
            ContainerLimits::from_cli("4g".to_string(), Some("invalid".to_string()), None, 1000);
        assert!(result.is_err());
    }

    // Memory bounds tests
    #[test]
    fn test_from_cli_memory_too_low() {
        let result = ContainerLimits::from_cli("100m".to_string(), None, None, 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("512 MB"));
    }

    #[test]
    fn test_from_cli_memory_too_high() {
        let result = ContainerLimits::from_cli("2000000m".to_string(), None, None, 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("1 TB"));
    }

    // CPU validation tests
    #[test]
    fn test_from_cli_invalid_cpu_format() {
        let result =
            ContainerLimits::from_cli("4g".to_string(), None, Some("abc".to_string()), 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid --cpus"));
    }

    #[test]
    fn test_from_cli_zero_cpus() {
        let result = ContainerLimits::from_cli("4g".to_string(), None, Some("0".to_string()), 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("positive"));
    }

    #[test]
    fn test_from_cli_negative_cpus() {
        let result =
            ContainerLimits::from_cli("4g".to_string(), None, Some("-2".to_string()), 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_cli_excessive_cpus() {
        let result =
            ContainerLimits::from_cli("4g".to_string(), None, Some("9999".to_string()), 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("1024"));
    }

    #[test]
    fn test_from_cli_valid_fractional_cpus() {
        let result =
            ContainerLimits::from_cli("4g".to_string(), None, Some("1.5".to_string()), 1000);
        assert!(result.is_ok());
    }

    // PID validation tests
    #[test]
    fn test_from_cli_zero_pids() {
        let result = ContainerLimits::from_cli("4g".to_string(), None, None, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("10"));
    }

    #[test]
    fn test_from_cli_excessive_pids() {
        let result = ContainerLimits::from_cli("4g".to_string(), None, None, 5_000_000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("1,000,000"));
    }

    #[test]
    fn test_from_cli_valid_pids() {
        let result = ContainerLimits::from_cli("4g".to_string(), None, None, 500);
        assert!(result.is_ok());
    }
}
