//! Memory-bounded crawl rate limiter for respectful web crawling
//!
//! This module provides a fast rate limiter using a token bucket algorithm
//! with per-domain tracking and LRU-based memory management. The implementation
//! bounds memory usage while maintaining lock-free token bucket operations.
//!
//! Key features:
//! - Async-friendly with `tokio::sync` primitives
//! - Thread-safe LRU cache with bounded capacity (max 1000 domains)
//! - Lock-free per-domain token bucket using atomic operations
//! - Automatic eviction of least-recently-used domains
//! - Safe for use with tokio multi-threaded runtime and task migration
//! - Per-domain rate limiting with independent token buckets
//! - Immediate Pass/Deny decisions with no blocking or sleep
//! - Fixed-point arithmetic for sub-token precision

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OnceCell};

/// Scaling factor for fixed-point token arithmetic (1000x precision)
const TOKEN_SCALE: u64 = 1000;

/// Scaling factor for nanosecond rate calculations
const RATE_SCALE: u64 = 1_000_000;

/// Rate limit decision for a crawl request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    /// Request is allowed to proceed
    Allow,
    /// Request should be denied/deferred due to rate limiting
    /// Contains the duration to wait before retrying
    Deny { retry_after: Duration },
}

/// Per-domain rate limiter using atomic token bucket algorithm
#[derive(Debug)]
struct DomainRateLimiter {
    /// Current available tokens scaled by `TOKEN_SCALE` for sub-token precision
    tokens: AtomicU64,
    /// Last token refill timestamp as nanoseconds since epoch
    last_refill_nanos: AtomicU64,
    /// Rate in tokens per nanosecond scaled by `TOKEN_SCALE` * `RATE_SCALE`
    rate_per_nano: u64,
    /// Maximum tokens scaled by `TOKEN_SCALE`
    max_tokens: u64,
}

impl DomainRateLimiter {
    /// Create a new domain rate limiter with the specified rate
    #[inline]
    fn new(rate_rps: f64) -> Self {
        let max_tokens = (rate_rps.max(1.0) * TOKEN_SCALE as f64) as u64;
        let rate_per_nano =
            ((rate_rps * TOKEN_SCALE as f64 * RATE_SCALE as f64) / 1_000_000_000.0) as u64;

        let now_nanos = Self::current_time_nanos();

        Self {
            tokens: AtomicU64::new(max_tokens),
            last_refill_nanos: AtomicU64::new(now_nanos),
            rate_per_nano,
            max_tokens,
        }
    }

    /// Get current time as nanoseconds since base time
    #[inline]
    fn current_time_nanos() -> u64 {
        // Use global base time for consistent time calculations across threads
        get_base_time().elapsed().as_nanos() as u64
    }

    /// Attempt to consume one token from the bucket
    #[inline]
    fn try_consume_token(&self) -> RateLimitDecision {
        let now_nanos = Self::current_time_nanos();

        // Refill tokens based on elapsed time
        self.refill_tokens(now_nanos);

        // Try to consume one token atomically
        loop {
            let current_tokens = self.tokens.load(Ordering::Relaxed);
            if current_tokens < TOKEN_SCALE {
                // Not enough tokens available - calculate wait time
                let tokens_needed = TOKEN_SCALE.saturating_sub(current_tokens);

                // Calculate nanoseconds needed to accumulate required tokens
                let nanos_needed = if self.rate_per_nano > 0 {
                    (tokens_needed.saturating_mul(RATE_SCALE)) / self.rate_per_nano
                } else {
                    1_000_000 // 1ms
                };

                let retry_after = Duration::from_nanos(nanos_needed);
                return RateLimitDecision::Deny { retry_after };
            }

            let new_tokens = current_tokens - TOKEN_SCALE;
            match self.tokens.compare_exchange_weak(
                current_tokens,
                new_tokens,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return RateLimitDecision::Allow,
                Err(_) => continue, // Retry on contention
            }
        }
    }

    /// Refill tokens based on elapsed time since last refill
    #[inline]
    fn refill_tokens(&self, now_nanos: u64) {
        loop {
            let last_refill = self.last_refill_nanos.load(Ordering::Relaxed);

            if now_nanos <= last_refill {
                break;
            }

            let elapsed_nanos = now_nanos.saturating_sub(last_refill);
            let tokens_to_add = (elapsed_nanos.saturating_mul(self.rate_per_nano)) / RATE_SCALE;

            if tokens_to_add == 0 {
                break;
            }

            match self.last_refill_nanos.compare_exchange_weak(
                last_refill,
                now_nanos,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    loop {
                        let current_tokens = self.tokens.load(Ordering::Relaxed);
                        let new_tokens = current_tokens
                            .saturating_add(tokens_to_add)
                            .min(self.max_tokens);

                        if current_tokens == new_tokens {
                            break;
                        }

                        match self.tokens.compare_exchange_weak(
                            current_tokens,
                            new_tokens,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => break,
                            Err(_) => continue,
                        }
                    }
                    break;
                }
                Err(_) => continue,
            }
        }
    }
}

/// Maximum number of domains to track simultaneously
const MAX_DOMAIN_LIMITERS: usize = 1000;

// Global shared state for domain rate limiters (async-friendly)
static DOMAIN_LIMITERS: OnceCell<Mutex<LruCache<String, Arc<DomainRateLimiter>>>> =
    OnceCell::const_new();

/// Get or initialize the domain limiters cache (async-friendly)
async fn get_domain_limiters() -> &'static Mutex<LruCache<String, Arc<DomainRateLimiter>>> {
    DOMAIN_LIMITERS
        .get_or_init(|| async {
            let capacity = unsafe { NonZeroUsize::new_unchecked(MAX_DOMAIN_LIMITERS) };
            Mutex::new(LruCache::new(capacity))
        })
        .await
}

/// Base time for all rate limit calculations (shared across threads)
static BASE_TIME: OnceLock<Instant> = OnceLock::new();

/// Get or initialize the base time  
#[inline]
fn get_base_time() -> &'static Instant {
    BASE_TIME.get_or_init(Instant::now)
}

/// Extract domain from URL
#[inline]
#[must_use]
pub fn extract_domain(url: &str) -> Option<String> {
    if let Some(scheme_end) = url.find("://") {
        let after_scheme = &url[scheme_end + 3..];
        let domain_end = after_scheme
            .find(['/', '?', '#', ':'])
            .unwrap_or(after_scheme.len());
        let domain = &after_scheme[..domain_end];
        let normalized = if domain.starts_with("www.") && domain.len() > 4 {
            &domain[4..]
        } else {
            domain
        };
        Some(normalized.to_lowercase())
    } else {
        let domain = url.split(['/', '?', '#', ':']).next().unwrap_or(url);
        let normalized = if domain.starts_with("www.") && domain.len() > 4 {
            &domain[4..]
        } else {
            domain
        };
        Some(normalized.to_lowercase())
    }
}

/// Get or create a rate limiter for the specified domain (async)
#[inline]
async fn get_domain_limiter(domain: &str, rate_rps: f64) -> RateLimitDecision {
    let cache_mutex = get_domain_limiters().await;
    let mut cache = cache_mutex.lock().await;

    if let Some(limiter) = cache.get(domain) {
        let limiter = Arc::clone(limiter);
        drop(cache);
        return limiter.try_consume_token();
    }

    let limiter = Arc::new(DomainRateLimiter::new(rate_rps));
    cache.put(domain.to_string(), Arc::clone(&limiter));
    drop(cache);
    limiter.try_consume_token()
}

/// Check if a crawl request to the given URL should be rate limited (async)
#[inline]
pub async fn check_crawl_rate_limit(url: &str, rate_rps: f64) -> RateLimitDecision {
    if rate_rps <= 0.0 {
        return RateLimitDecision::Allow;
    }

    let domain = match extract_domain(url) {
        Some(domain) if !domain.is_empty() => domain,
        _ => return RateLimitDecision::Allow,
    };

    get_domain_limiter(&domain, rate_rps).await
}

/// Check if an HTTP request should be rate limited (async)
#[inline]
pub async fn check_http_rate_limit(url: &str, rate_rps: f64) -> RateLimitDecision {
    check_crawl_rate_limit(url, rate_rps).await
}

/// Clear all domain rate limiters (async)
pub async fn clear_domain_limiters() {
    let cache_mutex = get_domain_limiters().await;
    let mut cache = cache_mutex.lock().await;
    cache.clear();
}

/// Get the number of domains currently being tracked for rate limiting (async)
pub async fn get_tracked_domain_count() -> usize {
    let cache_mutex = get_domain_limiters().await;
    let cache = cache_mutex.lock().await;
    cache.len()
}
