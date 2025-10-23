//! Configuration for search operations

use std::time::Duration;

/// Configuration for search operations
#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub cache_capacity: usize,
    pub concurrency_limit: usize,
    pub max_file_size: usize,
    pub max_repo_size: u64,
    pub fetch_timeout: Duration,
    pub api_timeout: Duration,
    pub rate_limit_buffer: u32,
    pub cache_ttl: Duration,
    pub api_page_size: u8,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            cache_capacity: 1000,
            concurrency_limit: 10,
            max_file_size: 10_485_760, // 10MB - allows large generated/minified files while preventing DoS
            max_repo_size: 1_073_741_824, // 1GB
            fetch_timeout: Duration::from_secs(30),
            api_timeout: Duration::from_secs(10),
            rate_limit_buffer: 100,
            cache_ttl: Duration::from_secs(3600), // 1 hour
            api_page_size: 100,                   // Maximum results per API page (GitHub API max)
        }
    }
}
