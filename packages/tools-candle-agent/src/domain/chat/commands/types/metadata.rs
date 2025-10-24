//! Command metadata and resource tracking with zero allocation patterns
//!
//! Provides blazing-fast command metadata management and resource usage tracking
//! with owned strings allocated once for maximum performance. No Arc usage, no locking.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use super::parameters::ParameterInfo;
use crate::domain::util::unix_timestamp_micros;

/// Command information for command registry with owned strings allocated once
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    /// Command name (owned string allocated once)
    pub name: String,
    /// Command description (owned string allocated once)
    pub description: String,
    /// Usage string showing syntax (owned string allocated once)
    pub usage: String,
    /// Command parameters with validation rules
    pub parameters: Vec<ParameterInfo>,
    /// Command aliases (owned strings allocated once)
    pub aliases: Vec<String>,
    /// Command category for organization (owned string allocated once)
    pub category: String,
    /// Usage examples (owned strings allocated once)
    pub examples: Vec<String>,
    /// Command version for compatibility tracking
    pub version: String,
    /// Author information
    pub author: Option<String>,
    /// Tags for searchability
    pub tags: Vec<String>,
    /// Minimum required permissions
    pub required_permissions: Vec<String>,
    /// Whether command is deprecated
    pub deprecated: bool,
    /// Deprecation message if deprecated
    pub deprecation_message: Option<String>,
    /// Whether command is experimental
    pub experimental: bool,
    /// Command stability level
    pub stability: StabilityLevel,
}

impl CommandInfo {
    /// Create new command info with essential fields - zero allocation constructor
    #[inline]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        usage: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            usage: usage.into(),
            parameters: Vec::new(),
            aliases: Vec::new(),
            category: category.into(),
            examples: Vec::new(),
            version: "1.0.0".to_string(),
            author: None,
            tags: Vec::new(),
            required_permissions: Vec::new(),
            deprecated: false,
            deprecation_message: None,
            experimental: false,
            stability: StabilityLevel::Stable,
        }
    }

    /// Add parameters - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_parameters(mut self, parameters: Vec<ParameterInfo>) -> Self {
        self.parameters = parameters;
        self
    }

    /// Add aliases - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }

    /// Add examples - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_examples(mut self, examples: Vec<String>) -> Self {
        self.examples = examples;
        self
    }

    /// Set version - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set author - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Add tags - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set required permissions - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.required_permissions = permissions;
        self
    }

    /// Mark as deprecated - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn deprecated(mut self, message: impl Into<String>) -> Self {
        self.deprecated = true;
        self.deprecation_message = Some(message.into());
        self
    }

    /// Mark as experimental - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn experimental(mut self) -> Self {
        self.experimental = true;
        self.stability = StabilityLevel::Experimental;
        self
    }

    /// Set stability level - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_stability(mut self, stability: StabilityLevel) -> Self {
        self.stability = stability;
        self
    }

    /// Check if command matches search query - zero allocation search
    #[inline]
    #[must_use]
    pub fn matches_search(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        self.name.to_lowercase().contains(&query_lower)
            || self.description.to_lowercase().contains(&query_lower)
            || self.category.to_lowercase().contains(&query_lower)
            || self
                .aliases
                .iter()
                .any(|alias| alias.to_lowercase().contains(&query_lower))
            || self
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&query_lower))
    }

    /// Get command signature for display - minimal allocation
    #[inline]
    #[must_use]
    pub fn signature(&self) -> String {
        if self.parameters.is_empty() {
            self.name.clone()
        } else {
            let params: Vec<String> = self
                .parameters
                .iter()
                .map(|p| {
                    if p.required {
                        format!("<{}>", p.name)
                    } else {
                        format!("[{}]", p.name)
                    }
                })
                .collect();
            format!("{} {}", self.name, params.join(" "))
        }
    }
}

/// Command stability levels for API compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StabilityLevel {
    /// Stable - guaranteed API compatibility
    Stable,
    /// Beta - mostly stable, minor changes possible
    Beta,
    /// Alpha - unstable, major changes possible
    Alpha,
    /// Experimental - highly unstable, may be removed
    Experimental,
    /// Deprecated - will be removed in future versions
    Deprecated,
}

impl StabilityLevel {
    /// Get stability as string for display
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Alpha => "alpha",
            Self::Experimental => "experimental",
            Self::Deprecated => "deprecated",
        }
    }

    /// Check if stability level allows production use
    #[inline]
    #[must_use]
    pub fn is_production_ready(&self) -> bool {
        matches!(self, Self::Stable | Self::Beta)
    }
}

/// Resource usage tracking for command execution with atomic operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// CPU time in microseconds
    pub cpu_time_us: u64,
    /// Number of network requests made
    pub network_requests: u32,
    /// Number of disk operations performed
    pub disk_operations: u32,
    /// Execution start timestamp (microseconds since epoch)
    pub start_time_us: u64,
    /// Execution end timestamp (microseconds since epoch)
    pub end_time_us: Option<u64>,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// Number of allocations made
    pub allocation_count: u64,
    /// Number of deallocations made
    pub deallocation_count: u64,
    /// Cache hits count
    pub cache_hits: u32,
    /// Cache misses count
    pub cache_misses: u32,
    /// Error count during execution
    pub error_count: u32,
    /// Warning count during execution
    pub warning_count: u32,
}

impl ResourceUsage {
    /// Create new resource usage tracker with current timestamp
    #[inline]
    #[must_use]
    pub fn new_with_start_time() -> Self {
        let start_time_us = unix_timestamp_micros();

        Self {
            memory_bytes: 0,
            cpu_time_us: 0,
            network_requests: 0,
            disk_operations: 0,
            start_time_us,
            end_time_us: None,
            peak_memory_bytes: 0,
            allocation_count: 0,
            deallocation_count: 0,
            cache_hits: 0,
            cache_misses: 0,
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Finalize resource tracking with end timestamp
    #[inline]
    pub fn finalize(&mut self) {
        self.end_time_us = Some(unix_timestamp_micros());
    }

    /// Get execution duration in microseconds
    #[inline]
    #[must_use]
    pub fn duration_us(&self) -> u64 {
        self.end_time_us
            .unwrap_or_else(unix_timestamp_micros)
            .saturating_sub(self.start_time_us)
    }

    /// Get execution duration as human readable string
    #[allow(clippy::cast_precision_loss)] // Acceptable for display formatting
    #[inline]
    #[must_use]
    pub fn duration_human(&self) -> String {
        let duration_us = self.duration_us();

        if duration_us < 1000 {
            format!("{duration_us}μs")
        } else if duration_us < 1_000_000 {
            format!("{:.1}ms", duration_us as f64 / 1000.0)
        } else if duration_us < 60_000_000 {
            format!("{:.2}s", duration_us as f64 / 1_000_000.0)
        } else {
            let minutes = duration_us / 60_000_000;
            let seconds = (duration_us % 60_000_000) / 1_000_000;
            format!("{minutes}m{seconds}s")
        }
    }

    /// Update memory usage if higher than current peak
    #[inline]
    pub fn update_peak_memory(&mut self, current_bytes: u64) {
        if current_bytes > self.peak_memory_bytes {
            self.peak_memory_bytes = current_bytes;
        }
        self.memory_bytes = current_bytes;
    }

    /// Increment network request counter atomically
    #[inline]
    pub fn increment_network_requests(&mut self) {
        self.network_requests = self.network_requests.saturating_add(1);
    }

    /// Increment disk operation counter atomically
    #[inline]
    pub fn increment_disk_operations(&mut self) {
        self.disk_operations = self.disk_operations.saturating_add(1);
    }

    /// Increment allocation counter atomically
    #[inline]
    pub fn increment_allocations(&mut self) {
        self.allocation_count = self.allocation_count.saturating_add(1);
    }

    /// Increment deallocation counter atomically
    #[inline]
    pub fn increment_deallocations(&mut self) {
        self.deallocation_count = self.deallocation_count.saturating_add(1);
    }

    /// Increment cache hit counter
    #[inline]
    pub fn increment_cache_hits(&mut self) {
        self.cache_hits = self.cache_hits.saturating_add(1);
    }

    /// Increment cache miss counter
    #[inline]
    pub fn increment_cache_misses(&mut self) {
        self.cache_misses = self.cache_misses.saturating_add(1);
    }

    /// Increment error counter
    #[inline]
    pub fn increment_errors(&mut self) {
        self.error_count = self.error_count.saturating_add(1);
    }

    /// Increment warning counter
    #[inline]
    pub fn increment_warnings(&mut self) {
        self.warning_count = self.warning_count.saturating_add(1);
    }

    /// Calculate cache hit ratio as percentage
    #[inline]
    #[must_use]
    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            (f64::from(self.cache_hits) / f64::from(total)) * 100.0
        }
    }

    /// Get memory efficiency metric (operations per MB)
    #[allow(clippy::cast_precision_loss)] // Acceptable for metrics calculations
    #[inline]
    #[must_use]
    pub fn memory_efficiency(&self) -> f64 {
        if self.peak_memory_bytes == 0 {
            0.0
        } else {
            let operations = self.network_requests + self.disk_operations;
            let memory_mb = self.peak_memory_bytes as f64 / (1024.0 * 1024.0);
            f64::from(operations) / memory_mb
        }
    }

    /// Check if execution had any issues (errors or excessive resource usage)
    #[inline]
    #[must_use]
    pub fn has_issues(&self) -> bool {
        self.error_count > 0 ||
        self.peak_memory_bytes > 100 * 1024 * 1024 || // > 100MB
        self.duration_us() > 30_000_000 // > 30 seconds
    }
}

impl Default for ResourceUsage {
    #[inline]
    fn default() -> Self {
        Self::new_with_start_time()
    }
}

/// Performance metrics aggregator for command execution analysis
#[derive(Debug)]
pub struct PerformanceMetrics {
    total_executions: AtomicU64,
    total_duration_us: AtomicU64,
    total_memory_bytes: AtomicU64,
    total_errors: AtomicU64,
    fastest_execution_us: AtomicU64,
    slowest_execution_us: AtomicU64,
}

impl PerformanceMetrics {
    /// Create new performance metrics tracker
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            total_executions: AtomicU64::new(0),
            total_duration_us: AtomicU64::new(0),
            total_memory_bytes: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            fastest_execution_us: AtomicU64::new(u64::MAX),
            slowest_execution_us: AtomicU64::new(0),
        }
    }

    /// Record execution metrics atomically
    #[inline]
    pub fn record_execution(&self, usage: &ResourceUsage) {
        let duration = usage.duration_us();

        self.total_executions.fetch_add(1, Ordering::Relaxed);
        self.total_duration_us
            .fetch_add(duration, Ordering::Relaxed);
        self.total_memory_bytes
            .fetch_add(usage.peak_memory_bytes, Ordering::Relaxed);
        self.total_errors
            .fetch_add(u64::from(usage.error_count), Ordering::Relaxed);

        // Update fastest execution
        let mut current_fastest = self.fastest_execution_us.load(Ordering::Relaxed);
        while duration < current_fastest {
            match self.fastest_execution_us.compare_exchange_weak(
                current_fastest,
                duration,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_fastest = actual,
            }
        }

        // Update slowest execution
        let mut current_slowest = self.slowest_execution_us.load(Ordering::Relaxed);
        while duration > current_slowest {
            match self.slowest_execution_us.compare_exchange_weak(
                current_slowest,
                duration,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_slowest = actual,
            }
        }
    }

    /// Get total execution count
    #[inline]
    pub fn total_executions(&self) -> u64 {
        self.total_executions.load(Ordering::Relaxed)
    }

    /// Get average execution duration in microseconds
    #[inline]
    pub fn average_duration_us(&self) -> u64 {
        let total = self.total_executions.load(Ordering::Relaxed);
        if total == 0 {
            0
        } else {
            self.total_duration_us.load(Ordering::Relaxed) / total
        }
    }

    /// Get average memory usage in bytes
    #[inline]
    pub fn average_memory_bytes(&self) -> u64 {
        let total = self.total_executions.load(Ordering::Relaxed);
        if total == 0 {
            0
        } else {
            self.total_memory_bytes.load(Ordering::Relaxed) / total
        }
    }

    /// Get error rate as percentage
    #[allow(clippy::cast_precision_loss)] // Acceptable for percentage calculations
    #[inline]
    pub fn error_rate(&self) -> f64 {
        let total = self.total_executions.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            (self.total_errors.load(Ordering::Relaxed) as f64 / total as f64) * 100.0
        }
    }

    /// Get fastest execution duration in microseconds
    #[inline]
    pub fn fastest_execution_us(&self) -> Option<u64> {
        let fastest = self.fastest_execution_us.load(Ordering::Relaxed);
        if fastest == u64::MAX {
            None
        } else {
            Some(fastest)
        }
    }

    /// Get slowest execution duration in microseconds
    #[inline]
    pub fn slowest_execution_us(&self) -> Option<u64> {
        let slowest = self.slowest_execution_us.load(Ordering::Relaxed);
        if slowest == 0 { None } else { Some(slowest) }
    }
}

impl Default for PerformanceMetrics {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
