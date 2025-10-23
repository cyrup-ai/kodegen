//! Crawl Engine Module
//!
//! This module contains the core crawling engine implementations that handle
//! the main crawling logic and orchestration. The functions in this module
//! provide both simple and progress-reporting crawling capabilities.

// Sub-modules
pub mod circuit_breaker;
pub mod cleanup;
pub mod core;
pub mod crawl_types;
pub mod crawler;
pub mod domain_limiter;
pub mod execution;
pub mod link_processor;
pub mod page_enhancer;
pub mod rate_limiter;

// Re-exports for public API
pub use execution::crawl_impl;

// Re-export core types for advanced usage
pub use core::{NoOpProgress, ProgressReporter, crawl_pages};

// Re-export rate limiter types
pub use rate_limiter::{RateLimitDecision, check_crawl_rate_limit, check_http_rate_limit};

// Re-export crawler types and functions
pub use crawler::{ChromiumoxideCrawler, extract_valid_urls, should_visit_url};

// Re-export circuit breaker types
pub use circuit_breaker::{CircuitBreaker, CircuitState, DomainHealth, extract_domain};

// Re-export domain limiter
pub use domain_limiter::DomainLimiter;

// Re-export crawl types
pub use crawl_types::{CrawlError, CrawlProgress, CrawlQueue, CrawlResult, Crawler};

// Re-export page enhancer
pub use page_enhancer::*;
