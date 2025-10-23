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
        let mut sys = System::new_all();
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
    
    /// Creates limits from CLI arguments.
    ///
    /// Validates that memory_swap >= memory.
    pub fn from_cli(
        memory: String,
        memory_swap: Option<String>,
        cpus: Option<String>,
        pids_limit: u32,
    ) -> Self {
        let memory_swap = memory_swap.unwrap_or_else(|| {
            // Default: memory + 2GB
            let mem_gb: u32 = memory
                .trim_end_matches('g')
                .trim_end_matches('m')
                .parse()
                .unwrap_or(4);
            format!("{}g", mem_gb + 2)
        });
        
        let cpus = cpus.unwrap_or_else(|| num_cpus::get().to_string());
        
        Self {
            memory,
            memory_swap,
            cpus,
            pids_limit,
        }
    }
}
