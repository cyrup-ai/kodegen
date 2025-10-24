//! Circuit breaker pattern for domain-level failure detection
//!
//! This module implements the circuit breaker pattern to detect consistently
//! failing domains and short-circuit further attempts, saving time and resources.
//!
//! The circuit breaker tracks domain health across three states:
//! - Closed: Normal operation, requests proceed
//! - Open: Too many failures, requests are blocked
//! - `HalfOpen`: Testing after cooldown period

use log::{debug, info, warn};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests proceed
    Closed,
    /// Testing after failures - limited requests allowed
    HalfOpen,
    /// Failing - skip requests to save resources
    Open,
}

/// Health tracking for a single domain
#[derive(Debug, Clone)]
pub struct DomainHealth {
    /// Number of consecutive failures without success
    pub consecutive_failures: u32,
    /// Total number of attempts made
    pub total_attempts: u32,
    /// Total number of successful requests
    pub total_successes: u32,
    /// Last time we had a successful request
    pub last_success: Option<Instant>,
    /// Time when circuit was opened (for timeout calculation)
    pub last_opened: Option<Instant>,
    /// Consecutive successes while in `HalfOpen` state
    pub consecutive_successes_in_halfopen: u32,
    /// Current circuit breaker state
    pub state: CircuitState,
}

impl DomainHealth {
    /// Create a new domain health tracker with default values
    fn new() -> Self {
        Self {
            consecutive_failures: 0,
            total_attempts: 0,
            total_successes: 0,
            last_success: None,
            last_opened: None,
            consecutive_successes_in_halfopen: 0,
            state: CircuitState::Closed,
        }
    }
}

/// Circuit breaker for tracking domain health and preventing wasted attempts
pub struct CircuitBreaker {
    /// Health tracking for each domain
    domains: HashMap<String, DomainHealth>,
    /// Number of consecutive failures before opening circuit
    failure_threshold: u32,
    /// Number of consecutive successes needed to close circuit from half-open
    success_threshold: u32,
    /// How long to wait before retrying a failed domain
    half_open_timeout: Duration,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the specified thresholds
    ///
    /// # Arguments
    /// * `failure_threshold` - Open circuit after this many consecutive failures
    /// * `success_threshold` - Close circuit after this many consecutive successes
    /// * `half_open_timeout` - Duration to wait before retrying failed domains
    #[must_use]
    pub fn new(
        failure_threshold: u32,
        success_threshold: u32,
        half_open_timeout: Duration,
    ) -> Self {
        Self {
            domains: HashMap::new(),
            failure_threshold,
            success_threshold,
            half_open_timeout,
        }
    }

    /// Check if we should attempt a request to the given domain
    ///
    /// Returns true if the request should proceed, false if it should be skipped.
    ///
    /// # Arguments
    /// * `domain` - The domain to check
    pub fn should_attempt(&mut self, domain: &str) -> bool {
        let health = self
            .domains
            .entry(domain.to_string())
            .or_insert_with(DomainHealth::new);

        match health.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if enough time has passed to retry
                // Use last_opened as the authoritative timestamp for timeout
                if let Some(opened) = health.last_opened {
                    if opened.elapsed() >= self.half_open_timeout {
                        health.state = CircuitState::HalfOpen;
                        health.consecutive_successes_in_halfopen = 0;
                        debug!("Circuit breaker transitioning to HALF-OPEN for domain: {domain}");
                        return true;
                    }
                } else {
                    // Circuit opened but timestamp not set (shouldn't happen with fix)
                    // Conservatively stay Open to avoid bypassing timeout
                    debug!(
                        "Circuit breaker OPEN with no timestamp for domain: {domain}, staying Open"
                    );
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful request to a domain
    ///
    /// This resets the consecutive failure count and may transition the circuit
    /// from `HalfOpen` to Closed state.
    ///
    /// # Arguments
    /// * `domain` - The domain that succeeded
    pub fn record_success(&mut self, domain: &str) {
        if let Some(health) = self.domains.get_mut(domain) {
            health.consecutive_failures = 0;
            health.total_successes += 1;
            health.total_attempts += 1;
            health.last_success = Some(Instant::now());

            if health.state == CircuitState::HalfOpen {
                health.consecutive_successes_in_halfopen += 1;

                if health.consecutive_successes_in_halfopen >= self.success_threshold {
                    health.state = CircuitState::Closed;
                    info!("Circuit breaker CLOSED for domain: {domain}");
                } else {
                    debug!(
                        "Circuit breaker HALF-OPEN success for domain: {} ({}/{})",
                        domain, health.consecutive_successes_in_halfopen, self.success_threshold
                    );
                }
            }
        }
    }

    /// Record a failed request to a domain
    ///
    /// This increments the failure count and may open the circuit if the
    /// threshold is exceeded.
    ///
    /// # Arguments
    /// * `domain` - The domain that failed
    /// * `error` - Description of the error for logging
    pub fn record_failure(&mut self, domain: &str, error: &str) {
        let health = self
            .domains
            .entry(domain.to_string())
            .or_insert_with(DomainHealth::new);

        health.consecutive_failures += 1;
        health.total_attempts += 1;

        if health.consecutive_failures >= self.failure_threshold
            && health.state != CircuitState::Open
        {
            health.state = CircuitState::Open;
            health.last_opened = Some(Instant::now());
            health.consecutive_successes_in_halfopen = 0;
            warn!(
                "Circuit breaker OPEN for domain {} after {} consecutive failures. Last error: {}",
                domain, health.consecutive_failures, error
            );
        } else if health.state != CircuitState::Open {
            debug!(
                "Circuit breaker failure for domain: {} ({}/{}): {}",
                domain, health.consecutive_failures, self.failure_threshold, error
            );
        }
    }

    /// Get health statistics for a domain
    ///
    /// Returns None if the domain has not been seen yet.
    ///
    /// # Arguments
    /// * `domain` - The domain to query
    #[must_use]
    pub fn get_health(&self, domain: &str) -> Option<&DomainHealth> {
        self.domains.get(domain)
    }

    /// Get health statistics for all tracked domains
    #[must_use]
    pub fn get_all_health(&self) -> &HashMap<String, DomainHealth> {
        &self.domains
    }
}

/// Extract domain from a URL string
///
/// Returns the host portion of the URL, or an error if the URL is invalid.
///
/// # Arguments
/// * `url_str` - The URL string to parse
pub fn extract_domain(url_str: &str) -> Result<String, String> {
    match url::Url::parse(url_str) {
        Ok(url) => {
            if let Some(host) = url.host_str() {
                Ok(host.to_string())
            } else {
                Err(format!("URL has no host: {url_str}"))
            }
        }
        Err(e) => Err(format!("Failed to parse URL {url_str}: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_closed_state() {
        let mut cb = CircuitBreaker::new(3, 2, Duration::from_secs(60));

        assert!(cb.should_attempt("example.com"));
        cb.record_success("example.com");

        let health = cb.get_health("example.com");
        assert!(health.is_some());
        let health = health.expect("Health should exist for example.com after recording success");
        assert_eq!(health.state, CircuitState::Closed);
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.total_successes, 1);
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let mut cb = CircuitBreaker::new(3, 2, Duration::from_secs(60));

        // First failure
        assert!(cb.should_attempt("example.com"));
        cb.record_failure("example.com", "test error");
        assert!(cb.should_attempt("example.com"));

        // Second failure
        cb.record_failure("example.com", "test error");
        assert!(cb.should_attempt("example.com"));

        // Third failure - should open circuit
        cb.record_failure("example.com", "test error");

        let health = cb.get_health("example.com");
        assert!(health.is_some());
        let health = health.expect("Health should exist for example.com after recording failures");
        assert_eq!(health.state, CircuitState::Open);
        assert_eq!(health.consecutive_failures, 3);

        // Should not allow requests while open
        assert!(!cb.should_attempt("example.com"));
    }

    #[test]
    fn test_circuit_breaker_half_open_after_timeout() {
        let mut cb = CircuitBreaker::new(2, 1, Duration::from_millis(100));

        // Cause failures to open circuit
        cb.record_failure("example.com", "test error");
        cb.record_failure("example.com", "test error");

        assert_eq!(
            cb.get_health("example.com")
                .expect("Health should exist after recording failures")
                .state,
            CircuitState::Open
        );

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));

        // Should transition to half-open
        assert!(cb.should_attempt("example.com"));
        assert_eq!(
            cb.get_health("example.com")
                .expect("Health should exist after timeout transition")
                .state,
            CircuitState::HalfOpen
        );
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path")
                .expect("Should extract domain from valid HTTPS URL"),
            "example.com"
        );
        assert_eq!(
            extract_domain("http://sub.example.com:8080/path?query=1")
                .expect("Should extract domain from URL with port and path"),
            "sub.example.com"
        );
        assert!(extract_domain("not a url").is_err());
    }
}
